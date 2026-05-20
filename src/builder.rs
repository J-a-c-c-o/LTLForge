use rustc_hash::FxHashMap;

use roxmltree::Document;

use crate::petri_net::{PetriNet, Place, Transition};

pub struct PetriNetBuilder {
    id: String,
    name: String,
    nodes: Vec<PetriNetNodeBuilder>,
    edges: Vec<PetriNetArcBuilder>,
    transition: Vec<PetriNetTransitionBuilder>,
}

struct PetriNetNodeBuilder {
    id: String,
    token: u32,
}

struct PetriNetTransitionBuilder {
    id: String,
}

struct PetriNetArcBuilder {
    source: String,
    target: String,
}

#[allow(dead_code)]
impl PetriNetBuilder {
    pub fn build_from_file(path: &str) -> Vec<PetriNet> {
        let xml = std::fs::read_to_string(path).expect("Failed to read XML file");
        Self::build_from_xml(&xml)
    }

    pub fn build_from_xml(xml: &str) -> Vec<PetriNet> {
        parse(xml)
    }

    pub fn new(id: String, name: String) -> Self {
        PetriNetBuilder {
            id,
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
            transition: Vec::new(),
        }
    }

    pub fn add_place(mut self, id: String, token: u32) -> Self {
        self.nodes.push(PetriNetNodeBuilder { id, token });
        self
    }

    pub fn add_transition(mut self, id: String) -> Self {
        self.transition.push(PetriNetTransitionBuilder { id });
        self
    }

    pub fn add_arc(mut self, source: String, target: String) -> Self {
        self.edges.push(PetriNetArcBuilder { source, target });
        self
    }

    pub fn build(self) -> PetriNet {
        let mut petri_net = PetriNet {
            id: self.id,
            name: self.name,
            places: Vec::new(),
            transitions: Vec::new(),
            initial_tokens: Vec::new(),
        };

        let mut place_indices: FxHashMap<String, u32> = FxHashMap::default();
        let mut transition_indices: FxHashMap<String, u32> = FxHashMap::default();

        for (index, node_builder) in self.nodes.into_iter().enumerate() {
            place_indices.insert(node_builder.id.clone(), index as u32);
            petri_net.places.push(Place {
                id: node_builder.id,
            });
            petri_net.initial_tokens.push(node_builder.token as u8);
        }

        for (index, transition_builder) in self.transition.into_iter().enumerate() {
            transition_indices.insert(transition_builder.id.clone(), index as u32);
            let transition = Transition {
                id: transition_builder.id,
                incoming: Vec::new(),
                outgoing: Vec::new(),
            };
            petri_net.transitions.push(transition);
        }

        for arc_builder in self.edges {
            if let (Some(&source_place), Some(&target_transition)) = (
                place_indices.get(&arc_builder.source),
                transition_indices.get(&arc_builder.target),
            ) {
                petri_net.transitions[target_transition as usize]
                    .incoming
                    .push(source_place);
            } else if let (Some(&source_transition), Some(&target_place)) = (
                transition_indices.get(&arc_builder.source),
                place_indices.get(&arc_builder.target),
            ) {
                petri_net.transitions[source_transition as usize]
                    .outgoing
                    .push(target_place);
            }
        }

        petri_net
    }
}

fn parse(xml: &str) -> Vec<PetriNet> {
    let doc = Document::parse(xml).unwrap();

    let mut nets: Vec<PetriNetBuilder> = Vec::new();
    let net_nodes = doc.descendants().filter(|n| n.has_tag_name("net"));
    for net_node in net_nodes {
        let net_id = net_node.attribute("id").unwrap_or("").to_string();
        let mut net = PetriNetBuilder {
            id: net_id,
            name: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            transition: Vec::new(),
        };
        net = parse_net(net_node, net);
        nets.push(net);
    }

    nets.into_iter().map(PetriNetBuilder::build).collect()
}

fn parse_net(net_node: roxmltree::Node, mut net: PetriNetBuilder) -> PetriNetBuilder {
    for child in net_node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "name" => {
                    net.name = parse_text(child);
                }
                "page" => {
                    net = parse_page(child, net);
                }
                _ => {}
            }
        }
    }
    net
}

fn parse_text(node: roxmltree::Node) -> String {
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "text" {
            return child.text().unwrap_or("").to_string();
        }
    }
    String::new()
}

fn parse_page(page_node: roxmltree::Node, mut net: PetriNetBuilder) -> PetriNetBuilder {
    for child in page_node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "place" => {
                    let place = parse_place(child);
                    net.nodes.push(place);
                }
                "transition" => {
                    let transition = parse_transition(child);
                    net.transition.push(transition);
                }
                "arc" => {
                    let arc = parse_arc(child);
                    net.edges.push(arc);
                }
                _ => {}
            }
        }
    }
    net
}

fn parse_place(place_node: roxmltree::Node) -> PetriNetNodeBuilder {
    let id = place_node.attribute("id").unwrap_or("").to_string();
    let mut token: u32 = 0;
    for child in place_node.children() {
        if child.is_element() && child.tag_name().name() == "initialMarking" {
            token = parse_text(child).parse().unwrap_or(0);
        }
    }
    PetriNetNodeBuilder { id, token }
}

fn parse_transition(transition_node: roxmltree::Node) -> PetriNetTransitionBuilder {
    let id = transition_node.attribute("id").unwrap_or("").to_string();
    PetriNetTransitionBuilder { id }
}

fn parse_arc(arc_node: roxmltree::Node) -> PetriNetArcBuilder {
    let source = arc_node.attribute("source").unwrap_or("").to_string();
    let target = arc_node.attribute("target").unwrap_or("").to_string();
    PetriNetArcBuilder { source, target }
}
