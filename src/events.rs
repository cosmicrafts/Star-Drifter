use bevy::prelude::*;
use rand::Rng;
use crate::factions::Faction;
use crate::game::GameData;

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
                handle_game_events,
                process_event_choices,
            ).in_set(EventSystemSet))
            .add_systems(Update, clear_consumed_input.after(crate::sector::NavigationSystemSet));
    }
}

#[derive(Message, Clone)]
pub struct GameEvent {
    pub _event_type: GameEventType,
    pub title: String,
    pub description: String,
    pub choices: Vec<EventChoice>,
    pub _faction: Option<Faction>,
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
    pub outcome: EventOutcome,
    pub requirements: Vec<EventRequirement>,
}

#[derive(Clone)]
pub enum EventOutcome {
    Combat { enemy_faction: Faction, difficulty: u32 },
    Reward { scrap: i32, fuel: f32, crew: Option<String> },
    Loss { scrap: i32, fuel: f32, hull_damage: f32 },
    FactionChange { faction: Faction, change: i32 },
    Discovery { item: String, description: String },
    Continue,
}

#[derive(Clone)]
pub enum EventRequirement {
    Fuel(f32),
    Scrap(u32),
    CrewSkill { _skill_type: String, _level: u32 },
}

#[derive(Resource, Default)]
pub struct ActiveEvent {
    pub event: Option<GameEvent>,
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
) {
    // Only trigger if no event is currently active
    if active_event.event.is_some() {
        return;
    }
    
    if let Some(sector) = sector_map.sectors.get(&sector_id) {
        if !sector.events.is_empty() {
            let mut rng = rand::thread_rng();
            let event_index = rng.gen_range(0..sector.events.len());
            let sector_event = &sector.events[event_index];
            
            let game_event = create_game_event_from_sector_event(sector_event, sector.danger_level);
            active_event.event = Some(game_event.clone());
            event_writer.write(game_event);
        } else {
            // Generate random encounter if sector has no predefined events
            let random_event = generate_random_event(sector.danger_level);
            active_event.event = Some(random_event.clone());
            event_writer.write(random_event);
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
    danger_level: u32,
) -> GameEvent {
    match sector_event.event_type {
        crate::sector::EventType::Encounter => {
            let faction = sector_event.faction.clone().unwrap_or(Faction::Spirats);
            GameEvent {
                _event_type: GameEventType::Combat,
                title: format!("{} Encounter", faction.name()),
                description: sector_event.description.clone(),
                choices: vec![
                    EventChoice {
                        text: "Engage in combat".to_string(),
                        outcome: EventOutcome::Combat { 
                            enemy_faction: faction.clone(), 
                            difficulty: danger_level 
                        },
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Attempt to negotiate".to_string(),
                        outcome: EventOutcome::FactionChange { 
                            faction: faction.clone(), 
                            change: 1 
                        },
                        requirements: vec![
                            EventRequirement::CrewSkill { 
                                _skill_type: "diplomacy".to_string(), 
                                _level: 2 
                            }
                        ],
                    },
                    EventChoice {
                        text: "Try to escape".to_string(),
                        outcome: EventOutcome::Loss { 
                            scrap: 0, 
                            fuel: 1.0, 
                            hull_damage: 0.0 
                        },
                        requirements: vec![
                            EventRequirement::Fuel(2.0),
                        ],
                    },
                    EventChoice {
                        text: "Ignore and continue".to_string(),
                        outcome: EventOutcome::Continue,
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
                choices: vec![
                    EventChoice {
                        text: "Investigate carefully".to_string(),
                        outcome: EventOutcome::Reward { 
                            scrap: 10 + (danger_level as i32 * 5), 
                            fuel: 0.0, 
                            crew: None 
                        },
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Quick salvage and leave".to_string(),
                        outcome: EventOutcome::Reward { 
                            scrap: 5, 
                            fuel: 0.0, 
                            crew: None 
                        },
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Ignore and continue".to_string(),
                        outcome: EventOutcome::Continue,
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
                choices: vec![
                    EventChoice {
                        text: "Offer assistance".to_string(),
                        outcome: EventOutcome::Reward { 
                            scrap: 0, 
                            fuel: 2.0, 
                            crew: Some("Grateful Survivor".to_string()) 
                        },
                        requirements: vec![
                            EventRequirement::Scrap(5),
                        ],
                    },
                    EventChoice {
                        text: "Demand payment first".to_string(),
                        outcome: EventOutcome::Reward { 
                            scrap: 15, 
                            fuel: 0.0, 
                            crew: None 
                        },
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Ignore the distress call".to_string(),
                        outcome: EventOutcome::Continue,
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
                choices: vec![
                    EventChoice {
                        text: "Navigate carefully".to_string(),
                        outcome: EventOutcome::Loss { 
                            scrap: 0, 
                            fuel: 1.0, 
                            hull_damage: 0.0 
                        },
                        requirements: vec![
                            EventRequirement::CrewSkill { 
                                _skill_type: "piloting".to_string(), 
                                _level: 2 
                            }
                        ],
                    },
                    EventChoice {
                        text: "Push through quickly".to_string(),
                        outcome: EventOutcome::Loss { 
                            scrap: 0, 
                            fuel: 0.5, 
                            hull_damage: 5.0 
                        },
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Find alternate route".to_string(),
                        outcome: EventOutcome::Loss { 
                            scrap: 0, 
                            fuel: 2.0, 
                            hull_damage: 0.0 
                        },
                        requirements: vec![
                            EventRequirement::Fuel(3.0),
                        ],
                    },
                    EventChoice {
                        text: "Avoid the hazard".to_string(),
                        outcome: EventOutcome::Continue,
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
                choices: vec![
                    EventChoice {
                        text: "Study the ancient technology".to_string(),
                        outcome: EventOutcome::Discovery { 
                            item: "Ancient Knowledge".to_string(),
                            description: "Your crew gains insight into advanced technologies.".to_string(),
                        },
                        requirements: vec![
                            EventRequirement::CrewSkill { 
                                _skill_type: "science".to_string(), 
                                _level: 3 
                            }
                        ],
                    },
                    EventChoice {
                        text: "Salvage what you can".to_string(),
                        outcome: EventOutcome::Reward { 
                            scrap: 20, 
                            fuel: 0.0, 
                            crew: None 
                        },
                        requirements: vec![],
                    },
                    EventChoice {
                        text: "Leave it undisturbed".to_string(),
                        outcome: EventOutcome::FactionChange { 
                            faction: faction.clone(), 
                            change: 2 
                        },
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
        choices: vec![
            EventChoice {
                text: "Trade scrap for fuel".to_string(),
                outcome: EventOutcome::Reward { 
                    scrap: -10, 
                    fuel: 3.0, 
                    crew: None 
                },
                requirements: vec![EventRequirement::Scrap(10)],
            },
            EventChoice {
                text: "Trade fuel for scrap".to_string(),
                outcome: EventOutcome::Reward { 
                    scrap: 15, 
                    fuel: -2.0, 
                    crew: None 
                },
                requirements: vec![EventRequirement::Fuel(2.0)],
            },
            EventChoice {
                text: "Decline and continue".to_string(),
                outcome: EventOutcome::Continue,
                requirements: vec![],
            },
        ],
        _faction: Some(Faction::Neutral),
    }
}

fn generate_anomaly_event(danger_level: u32) -> GameEvent {
    GameEvent {
        _event_type: GameEventType::Anomaly,
        title: "Cosmic Anomaly".to_string(),
        description: "Your sensors detect a strange energy signature ahead.".to_string(),
        choices: vec![
            EventChoice {
                text: "Investigate the anomaly".to_string(),
                outcome: EventOutcome::Reward { 
                    scrap: (danger_level as i32) * 8, 
                    fuel: 0.0, 
                    crew: None 
                },
                requirements: vec![],
            },
            EventChoice {
                text: "Scan from a safe distance".to_string(),
                outcome: EventOutcome::Reward { 
                    scrap: (danger_level as i32) * 3, 
                    fuel: 0.0, 
                    crew: None 
                },
                requirements: vec![
                    EventRequirement::CrewSkill { 
                        _skill_type: "sensors".to_string(), 
                        _level: 2 
                    }
                ],
            },
            EventChoice {
                text: "Ignore and continue".to_string(),
                outcome: EventOutcome::Continue,
                requirements: vec![],
            },
        ],
        _faction: None,
    }
}

fn generate_derelict_event(danger_level: u32) -> GameEvent {
    GameEvent {
        _event_type: GameEventType::Discovery,
        title: "Derelict Ship".to_string(),
        description: "You discover the wreckage of an ancient vessel drifting in space.".to_string(),
        choices: vec![
            EventChoice {
                text: "Board and explore".to_string(),
                outcome: EventOutcome::Reward { 
                    scrap: (danger_level as i32) * 6, 
                    fuel: 1.0, 
                    crew: None 
                },
                requirements: vec![],
            },
            EventChoice {
                text: "Salvage from outside".to_string(),
                outcome: EventOutcome::Reward { 
                    scrap: (danger_level as i32) * 3, 
                    fuel: 0.0, 
                    crew: None 
                },
                requirements: vec![],
            },
            EventChoice {
                text: "Leave it alone".to_string(),
                outcome: EventOutcome::Continue,
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
        choices: vec![
            EventChoice {
                text: "Fight the pirates".to_string(),
                outcome: EventOutcome::Combat { 
                    enemy_faction: Faction::Spirats, 
                    difficulty: danger_level + 1 
                },
                requirements: vec![],
            },
            EventChoice {
                text: "Pay tribute".to_string(),
                outcome: EventOutcome::Loss { 
                    scrap: (danger_level as i32) * 5, 
                    fuel: 0.0, 
                    hull_damage: 0.0 
                },
                requirements: vec![EventRequirement::Scrap((danger_level * 5) as u32)],
            },
            EventChoice {
                text: "Try to outrun them".to_string(),
                outcome: EventOutcome::Loss { 
                    scrap: 0, 
                    fuel: 2.0, 
                    hull_damage: 2.0 
                },
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
                outcome: EventOutcome::Continue,
                requirements: vec![],
            },
        ],
        _faction: Some(Faction::Spirats),
    }
}

fn generate_faction_event(danger_level: u32) -> GameEvent {
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
        choices: vec![
            EventChoice {
                text: "Hail them peacefully".to_string(),
                outcome: EventOutcome::FactionChange { 
                    faction: faction.clone(), 
                    change: 1 
                },
                requirements: vec![],
            },
            EventChoice {
                text: "Prepare for combat".to_string(),
                outcome: EventOutcome::Combat { 
                    enemy_faction: faction.clone(), 
                    difficulty: danger_level 
                },
                requirements: vec![],
            },
            EventChoice {
                text: "Try to avoid them".to_string(),
                outcome: EventOutcome::Loss { 
                    scrap: 0, 
                    fuel: 1.5, 
                    hull_damage: 0.0 
                },
                requirements: vec![EventRequirement::Fuel(2.0)],
            },
            EventChoice {
                text: "Ignore and continue".to_string(),
                outcome: EventOutcome::Continue,
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

fn process_event_choices(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut active_event: ResMut<ActiveEvent>,
    mut game_data: ResMut<GameData>,
    mut input_consumed: ResMut<InputConsumed>,
) {
    if let Some(event) = &active_event.event {
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
        }
        
        if let Some(choice_idx) = choice_selected {
            if let Some(key) = consumed_key {
                input_consumed.keys.push(key);
            }
            
            if choice_idx < event.choices.len() {
                let choice = &event.choices[choice_idx];
                
                // Check requirements
                let can_choose = check_requirements(&choice.requirements, &game_data);
                
                if can_choose {
                    apply_outcome(&choice.outcome, &mut game_data);
                    active_event.event = None;
                } else {
                    println!("Cannot choose this option - requirements not met!");
                }
            }
        }
    }
}

fn clear_consumed_input(mut input_consumed: ResMut<InputConsumed>) {
    input_consumed.keys.clear();
}

fn check_requirements(requirements: &[EventRequirement], game_data: &GameData) -> bool {
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

fn apply_outcome(outcome: &EventOutcome, game_data: &mut GameData) {
    match outcome {
        EventOutcome::Reward { scrap, fuel, crew } => {
            game_data.scrap = (game_data.scrap as i32 + scrap).max(0) as u32;
            game_data.fuel = (game_data.fuel + fuel).max(0.0);
            if let Some(crew_name) = crew {
                println!("New crew member joined: {}", crew_name);
                // TODO: Add crew member to game data
            }
        }
        EventOutcome::Loss { scrap, fuel, hull_damage } => {
            game_data.scrap = (game_data.scrap as i32 - scrap).max(0) as u32;
            game_data.fuel = (game_data.fuel - fuel).max(0.0);
            if *hull_damage > 0.0 {
                println!("Hull took {} damage!", hull_damage);
                // TODO: Apply hull damage to ship
            }
        }
        EventOutcome::Combat { enemy_faction, difficulty } => {
            println!("Combat initiated with {} (difficulty: {})!", enemy_faction.name(), difficulty);
            // TODO: Implement combat system
        }
        EventOutcome::FactionChange { faction, change } => {
            println!("Faction relation with {} changed by {}", faction.name(), change);
            // TODO: Update faction relations
        }
        EventOutcome::Discovery { item, description } => {
            println!("Discovery: {} - {}", item, description);
            // TODO: Add discovery to inventory/log
        }
        EventOutcome::Continue => {
            println!("You continue on your journey...");
        }
    }
}
