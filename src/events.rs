use bevy::prelude::*;
use rand::Rng;
use crate::factions::Faction;
use crate::game::{GameData, GameState};
use crate::llm::LlmRequestQueue;

pub struct EventsPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventSystemSet;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<GameEvent>()
            .insert_resource(ActiveEvent::default())
            .insert_resource(InputConsumed::default())
            .configure_sets(Update, EventSystemSet.before(crate::sector::NavigationSystemSet))
            .add_systems(Update, (
                handle_game_events.run_if(in_state(GameState::Playing)),
                process_event_choices.run_if(in_state(GameState::Playing)),
            ).in_set(EventSystemSet))
            .add_systems(Update, clear_consumed_input.run_if(in_state(GameState::Playing)).after(crate::sector::NavigationSystemSet));
    }
}

#[derive(Message, Clone)]
pub struct GameEvent {
    pub _event_type: GameEventType,
    pub title: String,
    pub description: String,
    pub choices: Vec<EventChoice>,
    pub _faction: Option<Faction>,
    pub current_stage: u32, // Track which stage we're on (0, 1, or 2)
    pub stages: Vec<GameEventStage>, // Pre-generated event stages
}

#[derive(Clone)]
pub struct GameEventStage {
    pub stage_number: u32,
    pub description: String,
    pub choices: Vec<StageChoice>, // Choices with pre-generated outcomes
}

#[derive(Clone)]
pub struct StageChoice {
    pub text: String,
    pub outcome_text: String, // Pre-generated narrative outcome
    pub fuel_delta: RewardRange,
    pub scrap_delta: RewardRange,
    pub hull_delta: RewardRange,
    pub next_stage: Option<u32>, // Which stage this leads to (None = event ends)
}

#[derive(Clone)]
pub enum RewardRange {
    Fixed(f32),
    Random { min: f32, max: f32 },
}

impl RewardRange {
    pub fn evaluate(&self) -> f32 {
        match self {
            RewardRange::Fixed(v) => *v,
            RewardRange::Random { min, max } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(*min..=*max)
            }
        }
    }
    
    pub fn evaluate_int(&self) -> i32 {
        self.evaluate() as i32
    }
}

#[derive(Clone)]
pub enum GameEventType {
    Combat,
    Diplomacy,
    Discovery,
    Hazard,
    Trade,
    Story,
    Anomaly,
}

#[derive(Clone)]
pub struct EventChoice {
    pub text: String,
    pub requirements: Vec<EventRequirement>,
}

#[derive(Clone, Debug)]
pub enum EventRequirement {
    Fuel(f32),
    Scrap(u32),
    CrewSkill { _skill_type: String, _level: u32 },
}

#[derive(Resource, Default)]
pub struct ActiveEvent {
    pub event: Option<GameEvent>,
    pub state: EventState,
    pub outcome_history: Vec<String>, // Track outcomes in this event
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum EventState {
    #[default]
    Initial,              // Just started, showing initial event
    AwaitingOutcome,      // Choice made, waiting for LLM outcome
    ShowingOutcome {      // Showing outcome, may have new choices
        outcome_text: String,
    },
    Complete,             // Event fully concluded
}

#[derive(Resource, Default)]
pub struct InputConsumed {
    pub keys: Vec<KeyCode>,
}

// Public function to trigger event for a sector (called automatically when arriving)
pub fn trigger_event_for_sector(
    sector_map: &crate::sector::SectorMap,
    sector_id: u32,
    event_writer: &mut MessageWriter<GameEvent>,
    active_event: &mut ActiveEvent,
    game_data: &GameData,
    llm_request_queue: Option<&mut LlmRequestQueue>,
    event_history: Option<&crate::llm::EventHistory>,
) {
    // Only trigger if no event is currently active
    if let Some(ref existing_event) = active_event.event {
        println!("[EVENT] trigger_event_for_sector: Skipping - event already active: \"{}\"", existing_event.title);
        return;
    }
    
    if let Some(sector) = sector_map.sectors.get(&sector_id) {
        println!("[EVENT] trigger_event_for_sector: Sector {} ({:?}) - {}", 
            sector_id, sector.sector_type, sector.name);
        
        if let Some(llm_queue) = llm_request_queue {
            println!("[EVENT] LLM queue available, requesting LLM-generated event");
            // Get event history
            let (recent_titles, total_events) = if let Some(history) = event_history {
                (history.recent_events.clone(), history.total_events)
            } else {
                (Vec::new(), 0)
            };
            
            let context = crate::llm::GameNarrativeContext {
                sector_name: sector.name.clone(),
                sector_type: sector.sector_type.clone(),
                danger_level: sector.danger_level,
                fuel: game_data.fuel,
                scrap: game_data.scrap,
                distance_traveled: sector_map.distance_traveled,
                recent_event_titles: recent_titles,
                total_events_seen: total_events,
            };
            
            llm_queue.0.push(crate::llm::LlmRequest::GenerateEvent { context });
            println!("[EVENT] LLM request queued for sector: {} ({})", sector_id, sector.name);
            println!("[EVENT] Waiting for LLM response...");
            
            // Don't show placeholder - just queue the request
            // The event will appear when LLM completes
            return;
        }
        
        println!("[EVENT] LLM not available, using fallback event system");
        // Fallback to old system if LLM is not available
        if !sector.events.is_empty() {
            println!("[EVENT] Using predefined sector event");
            let mut rng = rand::thread_rng();
            let event_index = rng.gen_range(0..sector.events.len());
            let sector_event = &sector.events[event_index];
            
            let game_event = create_game_event_from_sector_event(sector_event, sector.danger_level);
            println!("[EVENT] Setting fallback event: \"{}\"", game_event.title);
            active_event.event = Some(game_event.clone());
            event_writer.write(game_event);
            println!("[EVENT] Fallback event set and written");
        } else {
            // Generate random encounter if sector has no predefined events
            println!("[EVENT] Generating random event");
            let random_event = generate_random_event(sector.danger_level);
            println!("[EVENT] Setting random event: \"{}\"", random_event.title);
            active_event.event = Some(random_event.clone());
            event_writer.write(random_event);
            println!("[EVENT] Random event set and written");
        }
    }
}

// Old function - now disabled (events trigger automatically)
fn _trigger_sector_events(
    _event_writer: MessageWriter<GameEvent>,
    _sector_map: Res<crate::sector::SectorMap>,
    _active_event: ResMut<ActiveEvent>,
    _keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Disabled - events now trigger automatically when arriving at sectors
}

fn create_game_event_from_sector_event(
    sector_event: &crate::sector::SectorEvent,
    _danger_level: u32,
) -> GameEvent {
    match sector_event.event_type {
        crate::sector::EventType::Encounter => {
            let faction = sector_event.faction.clone().unwrap_or(Faction::Spirats);
            GameEvent {
                _event_type: GameEventType::Combat,
                title: format!("{} Encounter", faction.name()),
                description: sector_event.description.clone(),
                current_stage: 0,
                stages: vec![], // Fallback events don't use stages
                choices: vec![
                    EventChoice {
                        text: "Engage in combat".to_string(),
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Attempt to negotiate".to_string(),
                        requirements: vec![
                            EventRequirement::CrewSkill { 
                                _skill_type: "diplomacy".to_string(), 
                                _level: 2 
                            }
                        ],
                    },
                    EventChoice {
                        text: "Try to escape".to_string(),
                        requirements: vec![
                            EventRequirement::Fuel(2.0),
                        ],
                    },
                    EventChoice {
                        text: "Ignore and continue".to_string(),
                        requirements: vec![],
                    },
                ],
                _faction: Some(faction),
            }
        }
        crate::sector::EventType::Discovery => {
            GameEvent {
                _event_type: GameEventType::Discovery,
                title: "Discovery".to_string(),
                description: sector_event.description.clone(),
                current_stage: 0,
                stages: vec![],
                choices: vec![
                    EventChoice {
                        text: "Investigate carefully".to_string(),
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Quick salvage and leave".to_string(),
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Ignore and continue".to_string(),
                        requirements: vec![],
                    },
                ],
                _faction: sector_event.faction.clone(),
            }
        }
        crate::sector::EventType::Opportunity => {
            GameEvent {
                _event_type: GameEventType::Diplomacy,
                title: "Distress Call".to_string(),
                description: sector_event.description.clone(),
                current_stage: 0,
                stages: vec![],
                choices: vec![
                    EventChoice {
                        text: "Offer assistance".to_string(),
                        requirements: vec![
                            EventRequirement::Scrap(5),
                        ],
                    },
                    EventChoice {
                        text: "Demand payment first".to_string(),
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Ignore the distress call".to_string(),
                        requirements: vec![],
                    },
                ],
                _faction: None,
            }
        }
        crate::sector::EventType::Hazard => {
            GameEvent {
                _event_type: GameEventType::Hazard,
                title: "Space Hazard".to_string(),
                description: sector_event.description.clone(),
                current_stage: 0,
                stages: vec![],
                choices: vec![
                    EventChoice {
                        text: "Navigate carefully".to_string(),
                        requirements: vec![
                            EventRequirement::CrewSkill { 
                                _skill_type: "piloting".to_string(), 
                                _level: 2 
                            }
                        ],
                    },
                    EventChoice {
                        text: "Push through quickly".to_string(),
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Find alternate route".to_string(),
                        requirements: vec![
                            EventRequirement::Fuel(3.0),
                        ],
                    },
                    EventChoice {
                        text: "Avoid the hazard".to_string(),
                        requirements: vec![],
                    },
                ],
                _faction: None,
            }
        }
        crate::sector::EventType::Story => {
            let faction = sector_event.faction.clone().unwrap_or(Faction::Celestials);
            GameEvent {
                _event_type: GameEventType::Story,
                title: format!("{} Artifact", faction.name()),
                description: sector_event.description.clone(),
                current_stage: 0,
                stages: vec![],
                choices: vec![
                    EventChoice {
                        text: "Study the ancient technology".to_string(),
                        requirements: vec![
                            EventRequirement::CrewSkill { 
                                _skill_type: "science".to_string(), 
                                _level: 3 
                            }
                        ],
                    },
                    EventChoice {
                        text: "Salvage what you can".to_string(),
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Leave it undisturbed".to_string(),
                        requirements: vec![],
                    },
                ],
                _faction: Some(faction),
            }
        }
    }
}

fn generate_random_event(danger_level: u32) -> GameEvent {
    let mut rng = rand::thread_rng();
    
    match rng.gen_range(0..100) {
        0..=30 => generate_merchant_event(),
        31..=50 => generate_anomaly_event(danger_level),
        51..=70 => generate_derelict_event(danger_level),
        71..=85 => generate_pirate_event(danger_level),
        _ => generate_faction_event(danger_level),
    }
}

fn generate_merchant_event() -> GameEvent {
    GameEvent {
        _event_type: GameEventType::Trade,
        title: "Traveling Merchant".to_string(),
        description: "A merchant ship hails you, offering to trade supplies.".to_string(),
        current_stage: 0,
        stages: vec![],
        choices: vec![
            EventChoice {
                text: "Trade scrap for fuel".to_string(),
                requirements: vec![EventRequirement::Scrap(10)],
            },
            EventChoice {
                text: "Trade fuel for scrap".to_string(),
                requirements: vec![EventRequirement::Fuel(2.0)],
            },
            EventChoice {
                text: "Decline and continue".to_string(),
                requirements: vec![],
            },
        ],
        _faction: Some(Faction::Neutral),
    }
}

fn generate_anomaly_event(_danger_level: u32) -> GameEvent {
    GameEvent {
        _event_type: GameEventType::Anomaly,
        title: "Cosmic Anomaly".to_string(),
        description: "Your sensors detect a strange energy signature ahead.".to_string(),
        current_stage: 0,
        stages: vec![],
        choices: vec![
            EventChoice {
                text: "Investigate the anomaly".to_string(),
                requirements: vec![],
            },
            EventChoice {
                text: "Scan from a safe distance".to_string(),
                requirements: vec![
                    EventRequirement::CrewSkill { 
                        _skill_type: "sensors".to_string(), 
                        _level: 2 
                    }
                ],
            },
            EventChoice {
                text: "Ignore and continue".to_string(),
                requirements: vec![],
            },
        ],
        _faction: None,
    }
}

fn generate_derelict_event(_danger_level: u32) -> GameEvent {
    GameEvent {
        _event_type: GameEventType::Discovery,
        title: "Derelict Ship".to_string(),
        description: "You discover the wreckage of an ancient vessel drifting in space.".to_string(),
        current_stage: 0,
        stages: vec![],
        choices: vec![
            EventChoice {
                text: "Board and explore".to_string(),
                requirements: vec![],
            },
            EventChoice {
                text: "Salvage from outside".to_string(),
                requirements: vec![],
            },
            EventChoice {
                text: "Leave it alone".to_string(),
                requirements: vec![],
            },
        ],
        _faction: None,
    }
}

fn generate_pirate_event(danger_level: u32) -> GameEvent {
    GameEvent {
        _event_type: GameEventType::Combat,
        title: "Spirat Raiders".to_string(),
        description: "Spirat pirates emerge from an asteroid field, demanding tribute!".to_string(),
        current_stage: 0,
        stages: vec![],
        choices: vec![
            EventChoice {
                text: "Fight the pirates".to_string(),
                requirements: vec![],
            },
            EventChoice {
                text: "Pay tribute".to_string(),
                requirements: vec![EventRequirement::Scrap((danger_level * 5) as u32)],
            },
            EventChoice {
                text: "Try to outrun them".to_string(),
                requirements: vec![
                    EventRequirement::Fuel(3.0),
                    EventRequirement::CrewSkill { 
                        _skill_type: "engines".to_string(), 
                        _level: 2 
                    }
                ],
            },
            EventChoice {
                text: "Ignore and continue".to_string(),
                requirements: vec![],
            },
        ],
        _faction: Some(Faction::Spirats),
    }
}

fn generate_faction_event(_danger_level: u32) -> GameEvent {
    let mut rng = rand::thread_rng();
    let faction = match rng.gen_range(0..6) {
        0 => Faction::Cosmicons,
        1 => Faction::Spirats,
        2 => Faction::Webes,
        3 => Faction::Celestials,
        4 => Faction::Spades,
        _ => Faction::Archs,
    };

    GameEvent {
        _event_type: GameEventType::Diplomacy,
        title: format!("{} Patrol", faction.name()),
        description: format!("A {} patrol ship approaches your vessel.", faction.name()),
        current_stage: 0,
        stages: vec![],
        choices: vec![
            EventChoice {
                text: "Hail them peacefully".to_string(),
                requirements: vec![],
            },
            EventChoice {
                text: "Prepare for combat".to_string(),
                requirements: vec![],
            },
            EventChoice {
                text: "Try to avoid them".to_string(),
                requirements: vec![EventRequirement::Fuel(2.0)],
            },
            EventChoice {
                text: "Ignore and continue".to_string(),
                requirements: vec![],
            },
        ],
        _faction: Some(faction),
    }
}

fn handle_game_events(
    mut event_reader: MessageReader<GameEvent>,
    _active_event: ResMut<ActiveEvent>,
) {
    for event in event_reader.read() {
        println!("Event: {} - {}", event.title, event.description);
        for (i, choice) in event.choices.iter().enumerate() {
            println!("  {}: {}", i + 1, choice.text);
        }
    }
}

// Public function to process event choice by index (called from UI buttons or keyboard)
pub fn process_event_choice(
    choice_idx: usize,
    active_event: &mut ResMut<ActiveEvent>,
    game_data: &mut ResMut<GameData>,
) -> bool {
    // Check if we can process (need to borrow event first to check state)
    let can_process = {
        if let Some(event) = &active_event.event {
            // Don't process choices if we're awaiting an outcome
            if active_event.state == EventState::AwaitingOutcome {
                return false;
            }
            choice_idx < event.choices.len()
        } else {
            false
        }
    };
    
    if !can_process {
        return false;
    }
    
    // Now we can safely borrow mutably
    if let Some(event) = &mut active_event.event {
        let choice = &event.choices[choice_idx];
        
        // Check requirements
        let can_choose = check_requirements(&choice.requirements, &*game_data);
        
        if !can_choose {
            println!("Cannot choose this option - requirements not met!");
            return false;
        }
        
        // Use pre-generated outcomes from stages (no LLM calls during gameplay!)
        // Get current stage
        let current_stage_num = event.current_stage;
        let current_stage = event.stages.iter()
            .find(|s| s.stage_number == current_stage_num);
        
        if let Some(stage) = current_stage {
            if choice_idx >= stage.choices.len() {
                return false;
            }
            
            let stage_choice = &stage.choices[choice_idx];
            
            // Apply pre-generated outcome instantly (no LLM call!)
            println!("[EVENT] Processing choice: {}", stage_choice.text);
            println!("[EVENT] Outcome: {}", stage_choice.outcome_text);
            
            // Evaluate random ranges
            let fuel_delta = stage_choice.fuel_delta.evaluate();
            let scrap_delta = stage_choice.scrap_delta.evaluate_int();
            let hull_delta = stage_choice.hull_delta.evaluate_int();
            
            println!("[EVENT] Stat changes: fuel={:.1}, scrap={}, hull={}", fuel_delta, scrap_delta, hull_delta);
            
            // Apply stat changes
            game_data.fuel += fuel_delta;
            game_data.scrap = (game_data.scrap as i32 + scrap_delta).max(0) as u32;
            // TODO: Apply hull damage if we track hull
            
            // Store outcome text and next stage info before dropping event borrow
            let outcome_text = stage_choice.outcome_text.clone();
            let next_stage_num = stage_choice.next_stage;
            let next_stage_data = next_stage_num.and_then(|n| {
                event.stages.iter()
                    .find(|s| s.stage_number == n)
                    .map(|s| (n, s.description.clone(), s.choices.clone()))
            });
            
            // Update event description to show outcome
            event.description = format!("{}\n\n{}", event.description, outcome_text.clone());
            
            // Drop event borrow, then access active_event
            let outcome_text_for_history = outcome_text.clone();
            let outcome_text_for_state = outcome_text.clone();
            
            // Add outcome to history (now we can borrow active_event)
            active_event.outcome_history.push(outcome_text_for_history);
            
            // Check if event continues to next stage
            if let Some((next_stage_num, next_stage_desc, next_stage_choices)) = next_stage_data {
                // Navigate to next stage
                if let Some(event) = &mut active_event.event {
                    event.current_stage = next_stage_num;
                    event.description = format!("{}\n\n{}", event.description, next_stage_desc);
                    
                    // Convert next stage choices to EventChoice format
                    event.choices = next_stage_choices.iter().map(|sc| {
                        EventChoice {
                            text: sc.text.clone(),
 // Placeholder
                            requirements: vec![],
                        }
                    }).collect();
                }
                
                active_event.state = EventState::ShowingOutcome {
                    outcome_text: outcome_text_for_state,
                };
                
                println!("[EVENT] Moving to stage {}", next_stage_num);
                return true;
            } else {
                // Event ends
                active_event.state = EventState::Complete;
                active_event.event = None;
                println!("[EVENT] Event concluded");
                return true;
            }
        } else {
            println!("[EVENT] Warning: Current stage {} not found", current_stage_num);
            active_event.state = EventState::Complete;
            active_event.event = None;
            return false;
        }
    }
    false
}

fn process_event_choices(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut active_event: ResMut<ActiveEvent>,
    mut game_data: ResMut<GameData>,
    mut input_consumed: ResMut<InputConsumed>,
) {
    if let Some(_event) = &active_event.event {
        let mut choice_selected = None;
        let mut consumed_key = None;
        
        if keyboard.just_pressed(KeyCode::Digit1) {
            choice_selected = Some(0);
            consumed_key = Some(KeyCode::Digit1);
        } else if keyboard.just_pressed(KeyCode::Digit2) {
            choice_selected = Some(1);
            consumed_key = Some(KeyCode::Digit2);
        } else if keyboard.just_pressed(KeyCode::Digit3) {
            choice_selected = Some(2);
            consumed_key = Some(KeyCode::Digit3);
        } else if keyboard.just_pressed(KeyCode::Digit4) {
            choice_selected = Some(3);
            consumed_key = Some(KeyCode::Digit4);
        }
        
        if let Some(choice_idx) = choice_selected {
            if let Some(key) = consumed_key {
                input_consumed.keys.push(key);
            }
            
            process_event_choice(
                choice_idx, 
                &mut active_event, 
                &mut game_data,
            );
        }
    }
}

fn clear_consumed_input(mut input_consumed: ResMut<InputConsumed>) {
    input_consumed.keys.clear();
}

pub fn check_requirements(requirements: &[EventRequirement], game_data: &GameData) -> bool {
    for requirement in requirements {
        match requirement {
            EventRequirement::Fuel(amount) => {
                if game_data.fuel < *amount {
                    return false;
                }
            }
            EventRequirement::Scrap(amount) => {
                if game_data.scrap < *amount {
                    return false;
                }
            }
            EventRequirement::CrewSkill { _skill_type: _, _level: _ } => {
                // TODO: Implement crew skill checking
            }
        }
    }
    true
}

