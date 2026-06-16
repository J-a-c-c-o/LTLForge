#!/bin/bash

# LTL Test Runner Script with Typst Output Generator
# Usage: ./run_ltl_tests.sh [--timeout SECONDS] [--input-dir DIR] [--output-file FILE] [--verbose]

set -e

# Configuration
TIMEOUT=60
INPUT_DIR="inputs/test"
OUTPUT_FILE="results.txt"
TYPST_FILE="results.typ"
MEMORY_LIMIT="24576"
VERBOSE=false
LTL_PATTERNS=("LTLCardinality.txt" "LTLFireability.txt")

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --timeout)
            TIMEOUT=$2
            shift 2
            ;;
        --memory-limit)
            MEMORY_LIMIT=$2
            shift 2
            ;;
        --input-dir)
            INPUT_DIR=$2
            shift 2
            ;;
        --output-file)
            OUTPUT_FILE=$2
            OUTPUT_FILE_DIR=$(dirname "$OUTPUT_FILE")
            TYPST_FILE="${OUTPUT_FILE_DIR}/$(basename "$OUTPUT_FILE" .txt).typ"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--timeout SECONDS] [--memory-limit MB] [--input-dir DIR] [--output-file FILE] [--verbose]"
            exit 1
            ;;
    esac
done

# Check if input directory exists
if [ ! -d "$INPUT_DIR" ]; then
    echo "[ERROR] Input directory does not exist: $INPUT_DIR"
    exit 1
fi

# Build the project if not already built
if [ ! -f "target/release/ltlforge" ]; then
    echo "Building project..."
    cargo build --release
fi

BINARY="target/release/ltlforge"

# Initialize text output file
{
    echo "LTL Model Checking Results"
    echo "=========================="
    echo "Generated: $(date)"
    echo "Memory Limit: ${MEMORY_LIMIT} MB"
    echo "Timeout: ${TIMEOUT}s"
    echo "Input Directory: $INPUT_DIR"
    echo ""
} > "$OUTPUT_FILE"

# Initialize Typst output file with boilerplate definitions
{
    echo '#set page(paper: "a4", flipped: false, margin: 1.2cm)'
    echo '#set text(font: "Liberation Sans", size: 9.5pt)'
    echo ""
    echo "// Styling definitions"
    echo '#show table.cell.where(y: 0): set text(weight: "bold")'
    echo '#let mono(body) = text(font: "Liberation Mono", size: 8.5pt, tracking: 1pt, body)'
    echo '#let filler = mono("----------------")'
    echo ""
    echo "#grid("
    echo "  columns: (1.1fr, 0.9fr),"
    echo "  gutter: 0.5cm,"
    echo "  table("
    echo "    columns: (1fr, 3fr),"
    echo "    align: (center, left),"
    echo "    fill: (x, y) => if y == 0 { rgb(\"eeeeee\") } else { none },"
    echo "    [Raw Character], [Meaning / Explanation],"
    echo '    mono("p"), [Property Holds (Passed)],'
    echo '    mono("f"), [Property Violated (Failed)],'
    echo '    mono("M"), [Out of Memory (OOM Error)],'
    echo '    mono("T"), [Out of Time (Timeout)],'
    echo "  ),"
    echo "  table("
    echo "    columns: (1fr, 2fr),"
    echo "    align: (center, left),"
    echo "    fill: (x, y) => if y == 0 { rgb(\"eeeeee\") } else { none },"
    echo "    [Verification Status], [Definition],"
    echo '    mono("T"), [Correct (Verdict matches the expected result)],'
    echo '    mono("X"), [Fail (Verdict does *not* match the expected result)],'
    echo '    mono("-"), [Platform limitation encountered (OOM or Timeout)],'
    echo "  )"
    echo ")"
    echo ""
} > "$TYPST_FILE"

# Counter variables
total_tests=0
passed_tests=0
failed_tests=0
error_tests=0
processed_dirs=0

# Create a temporary file to collect table rows
TMP_ROWS=$(mktemp)

# Process each test case directory
for test_dir in "$INPUT_DIR"/*/ ; do
    if [ ! -d "$test_dir" ]; then
        continue
    fi
    
    ((processed_dirs++)) || true
    test_name=$(basename "$test_dir")
    model_file="$test_dir/model.pnml"
    
    if [ ! -f "$model_file" ]; then
        echo "[WARNING] Skipping $test_name: Missing model.pnml"
        continue
    fi
    
    # Try to extract family name (e.g., "Parking" from "Parking-PT-104")
    if [[ "$test_name" =~ ^([^-]+) ]]; then
        family_name="${BASH_REMATCH[1]}"
    else
        family_name="Unknown"
    fi
    
    # Process each LTL test file
    for pattern in "${LTL_PATTERNS[@]}"; do
        ltl_file="$test_dir/$pattern"
        
        if [ ! -f "$ltl_file" ]; then
            if [ "$VERBOSE" = true ]; then
                echo "[INFO] Skipping $pattern in $test_name: File not found"
            fi
            continue
        fi
        
        # Extract test type from pattern
        test_type="${pattern%.txt}"
        display_type="${test_type#LTL}"
        result_name="${test_name}-${display_type}"
        
        if [ "$VERBOSE" = true ]; then
            echo "Running: $result_name"
        fi
        
        # Run the model checker
        set +e
        result=$("$BINARY" check "$model_file" "$ltl_file" --timeout "$TIMEOUT" --memory-limit "$MEMORY_LIMIT" --simple 2>&1)
        exit_code=$?
        set -e

        if [ "$VERBOSE" = true ]; then
            echo "Raw output for $result_name: $result"
        fi
        
        # Clean result strings
        result_stripped=$(echo "$result" | tr -d '\n' | xargs)
        clean_result=$(echo "$result_stripped" | tr -cd 'pfMT')
        
        if [ -z "$clean_result" ]; then
            clean_result="MMMMMMMMMMMMMMMM" 
        fi

        # Counters logic
        pass_count=$(echo "$clean_result" | tr -cd 'p' | wc -c)
        fail_count=$(echo "$clean_result" | tr -cd 'f' | wc -c)
        mem_error_count=$(echo "$clean_result" | tr -cd 'M' | wc -c)
        timeout_count=$(echo "$clean_result" | tr -cd 'T' | wc -c)
        
        ((total_tests++)) || true
        ((passed_tests += pass_count)) || true
        ((failed_tests += fail_count)) || true
        ((error_tests += mem_error_count)) || true
        ((error_tests += timeout_count)) || true
        
        # Append to TXT output file
        {
            echo "$result_name"
            echo "  Raw Output: $result_stripped"
            echo "  Pass: $pass_count, Fail: $fail_count, Memory Error: $mem_error_count, Timeout: $timeout_count"
            echo ""
        } >> "$OUTPUT_FILE"

        # Save structured format to temp file
        echo "${family_name}:::[${test_name}], [${display_type}], mono(\"${clean_result}\"), filler, filler," >> "$TMP_ROWS"
    done
done

# Group temp rows into final Typst structure
if [ -s "$TMP_ROWS" ]; then
    current_family=""
    
    # Process substitution keeps this loop running in the current shell context
    while IFS= read -r line; do
        family_name=$(echo "$line" | awk -F':::' '{print $1}')
        row_content=$(echo "$line" | awk -F':::' '{print $2}')
        
        if [ "$family_name" != "$current_family" ]; then
            # If a family block was already open, close it!
            if [ -n "$current_family" ]; then
                {
                    echo "  ),"
                    echo "  caption: [Formula Breakdown - ${current_family} Family]"
                    echo ")"
                    echo ""
                } >> "$TYPST_FILE"
            fi
            
            current_family="$family_name"
            
            {
                echo "== ${current_family} Family"
                echo ""
                echo "#figure("
                echo "  table("
                echo "    columns: (4.5fr, 1.6fr, 3fr, 3fr, 3fr),"
                echo "    align: (left, center, center, center, center),"
                echo "    fill: (x, y) => if y == 0 { rgb(\"f5f5f5\") } else { none },"
                echo "    [Instance], [Type], [Raw Evaluation Results], [Expected Results], [Is Correct?],"
                echo "    "
            } >> "$TYPST_FILE"
        fi
        
        echo "    ${row_content}" >> "$TYPST_FILE"
    done < <(sort "$TMP_ROWS")
    
    # Explicitly catch and close the absolute last group in the file
    if [ -n "$current_family" ]; then
        {
            echo "  ),"
            echo "  caption: [Formula Breakdown - ${current_family} Family]"
            echo ")"
            echo ""
        } >> "$TYPST_FILE"
    fi
fi

rm -f "$TMP_ROWS"

if [ "$processed_dirs" -eq 0 ]; then
    echo "[WARNING] No directories were found in $INPUT_DIR!"
fi

# Write summary to outputs
summary_block="==========================
Summary:
Total test files processed: $total_tests
Total properties evaluated:
  Passed: $passed_tests
  Failed: $failed_tests
  Error/Timeout: $error_tests"

echo "$summary_block" >> "$OUTPUT_FILE"
echo ""
echo "$summary_block"
echo ""
echo "Results saved to: $OUTPUT_FILE"
echo "Typst code generated: $TYPST_FILE"