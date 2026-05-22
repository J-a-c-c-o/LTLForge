use rustc_hash::FxHashMap;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PetriNet {
    pub id: String,
    pub name: String,
    pub places: Vec<Place>,
    pub transitions: Vec<Transition>,
    pub initial_tokens: Vec<u32>,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Place {
    pub id: String,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Transition {
    pub id: String,
    pub incoming: Vec<u32>,
    pub outgoing: Vec<u32>,
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PetriState {
    pub tokens: Vec<u32>,
}

impl PetriNet {
    pub fn initial_state(&self) -> PetriState {
        PetriState {
            tokens: self.initial_tokens.clone(),
        }
    }

    pub fn next_states(&self, state: &PetriState) -> Vec<PetriState> {
        let mut next_states = Vec::new();

        for transition in &self.transitions {
            if transition.is_fireable(state) {
                let mut next_tokens = state.tokens.clone();

                for &incoming_index in &transition.incoming {
                    let index = incoming_index as usize;
                    next_tokens[index] = next_tokens[index].saturating_sub(1);
                }

                for &outgoing_index in &transition.outgoing {
                    let index = outgoing_index as usize;
                    next_tokens[index] = next_tokens[index].saturating_add(1);
                }

                next_states.push(PetriState {
                    tokens: next_tokens,
                });
            }
        }

        next_states
    }

    pub fn get_state_string(&self, state: &PetriState) -> String {
        // print id = tokens of each place in order, separated by commas
        let places_string = self
            .places
            .iter()
            .enumerate()
            .map(|(i, place)| format!("{}:{}", place.id, state.tokens.get(i).copied().unwrap_or(0)))
            .collect::<Vec<String>>()
            .join(",");

        format!("{{{}}}", places_string)
    }

    pub fn to_pnml(&self) -> String {
        let mut pnml = String::new();
        pnml.push_str(&format!(
            "<pnml><net id=\"{}\" type=\"http://www.pnml.org/version-2009/grammar/ptnet\">\n",
            self.id
        ));
        pnml.push_str(&format!("<name><text>{}</text></name>\n", self.name));
        pnml.push_str("<page id=\"page1\">\n");

        for place in &self.places {
            pnml.push_str(&format!("<place id=\"{}\">\n", place.id));
            if let Some(index) = self.places.iter().position(|p| p.id == place.id) {
                let initial_tokens = self.initial_tokens.get(index).copied().unwrap_or(0);
                pnml.push_str(&format!(
                    "<initialMarking><text>{}</text></initialMarking>\n",
                    initial_tokens
                ));
            }
            pnml.push_str("</place>\n");
        }

        for transition in &self.transitions {
            pnml.push_str(&format!("<transition id=\"{}\" />\n", transition.id));
        }

        for transition in &self.transitions {
            for &incoming_index in &transition.incoming {
                if let Some(place) = self.places.get(incoming_index as usize) {
                    pnml.push_str(&format!(
                        "<arc id=\"{}\" source=\"{}\" target=\"{}\" />\n",
                        format_args!("arc_{}_{}", place.id, transition.id),
                        place.id,
                        transition.id
                    ));
                }
            }
            for &outgoing_index in &transition.outgoing {
                if let Some(place) = self.places.get(outgoing_index as usize) {
                    pnml.push_str(&format!(
                        "<arc id=\"{}\" source=\"{}\" target=\"{}\" />\n",
                        format_args!("arc_{}_{}", transition.id, place.id),
                        transition.id,
                        place.id
                    ));
                }
            }
        }

        pnml.push_str("</page>");
        pnml.push_str("</net></pnml>");
        pnml
    }
}

impl Transition {
    pub fn is_fireable(&self, state: &PetriState) -> bool {
        self.is_fireable_tokens(&state.tokens)
    }

    pub fn is_fireable_tokens(&self, tokens: &[u32]) -> bool {
        let mut required_tokens: FxHashMap<u32, u32> = FxHashMap::default();
        for &incoming_index in &self.incoming {
            *required_tokens.entry(incoming_index).or_insert(0) += 1;
        }

        required_tokens.into_iter().all(|(incoming_index, required_count)| {
            tokens
                .get(incoming_index as usize)
                .copied()
                .unwrap_or(0)
                >= required_count
        })
    }
}

// test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_petri_net() {
        let petri_net = PetriNet {
            id: "test_net".to_string(),
            name: "Test Net".to_string(),
            places: vec![
                Place {
                    id: "p1".to_string(),
                },
                Place {
                    id: "p2".to_string(),
                },
            ],
            transitions: vec![Transition {
                id: "t1".to_string(),
                incoming: vec![0],
                outgoing: vec![1],
            }],
            initial_tokens: vec![1, 0],
        };
        let initial_state = petri_net.initial_state();
        assert_eq!(initial_state.tokens, vec![1, 0]);
        let next_states = petri_net.next_states(&initial_state);
        assert_eq!(next_states.len(), 1);
        assert_eq!(next_states[0].tokens, vec![0, 1]);
    }
}
