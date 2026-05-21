#!/bin/bash

# LTL Test Runner Script
# Usage: ./run_ltl_tests.sh [--timeout SECONDS] [--input-dir DIR] [--output-file FILE] [--verbose]

set -e

# Configuration
TIMEOUT=300
INPUT_DIR="inputs/test"
OUTPUT_FILE="results.txt"
MEMORY_LIMIT="4096"
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
if [ ! -f "target/release/ltltools" ]; then
    echo "Building project..."
    cargo build --release
fi

BINARY="target/release/ltltools"

# Initialize output file with timestamp
{
    echo "LTL Model Checking Results"
    echo "=========================="
    echo "Generated: $(date)"
    echo "Memory Limit: ${MEMORY_LIMIT} MB"
    echo "Timeout: ${TIMEOUT}s"
    echo "Input Directory: $INPUT_DIR"
    echo ""
} > "$OUTPUT_FILE"

# Counter variables
total_tests=0
passed_tests=0
failed_tests=0
error_tests=0
processed_dirs=0

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
        result_name="${test_name}-${test_type}"
        
        if [ "$VERBOSE" = true ]; then
            echo "Running: $result_name"
        fi
        
        # Run the model checker with simple output and capture result
        set +e
        result=$("$BINARY" check "$model_file" "$ltl_file" --timeout "$TIMEOUT" --memory-limit "$MEMORY_LIMIT" --simple 2>&1)
        exit_code=$?
        set -e

        if [ "$VERBOSE" = true ]; then
            echo "Raw output for $result_name: $result"
        fi
        
        # Remove any trailing whitespace/newlines
        result=$(echo "$result" | tr -d '\n' | xargs)
        
        # Extract ONLY the characters p, f, T, M from the result
        clean_result=$(echo "$result" | tr -cd 'pfMT')
        
        if [ -n "$clean_result" ] && [ $exit_code -eq 0 ]; then
            pass_count=$(echo "$clean_result" | tr -cd 'p' | wc -c)
            fail_count=$(echo "$clean_result" | tr -cd 'f' | wc -c)
            mem_error_count=$(echo "$clean_result" | tr -cd 'M' | wc -c)
            timeout_count=$(echo "$clean_result" | tr -cd 'T' | wc -c)
            
            ((total_tests++)) || true
            ((passed_tests += pass_count)) || true
            ((failed_tests += fail_count)) || true
            ((error_tests += mem_error_count)) || true
            ((error_tests += timeout_count)) || true
            
            # Write to output file
            {
                echo "$result_name"
                echo "  Raw Output: $result"
                echo "  Pass: $pass_count, Fail: $fail_count, Memory Error: $mem_error_count, Timeout: $timeout_count"
                echo ""
            } >> "$OUTPUT_FILE"
        else
            # Error occurred or no valid output was found
            {
                echo "$result_name"
                echo "  Error/Crash Output: $result"
                echo ""
            } >> "$OUTPUT_FILE"
            
            ((total_tests++)) || true
            ((error_tests++)) || true
        fi
    done
done

if [ "$processed_dirs" -eq 0 ]; then
    echo "[WARNING] No directories were found in $INPUT_DIR!"
fi

# Write summary to output file and stdout
{
    echo "=========================="
    echo "Summary:"
    echo "Total test files processed: $total_tests"
    echo "Total properties evaluated:"
    echo "  Passed: $passed_tests"
    echo "  Failed: $failed_tests"
    echo "  Error/Timeout: $error_tests"
} >> "$OUTPUT_FILE"

echo ""
echo "=========================="
echo "Summary:"
echo "Total test files processed: $total_tests"
echo "Total properties evaluated:"
echo "  Passed: $passed_tests"
echo "  Failed: $failed_tests"
echo "  Error/Timeout: $error_tests"
echo ""
echo "Results saved to: $OUTPUT_FILE"