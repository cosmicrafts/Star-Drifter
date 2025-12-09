use bevy::prelude::*;
use serde_json::{Value, json};
use crate::sector::SectorType;
use crate::events::{GameEvent, GameEventType, EventChoice, EventOutcome};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct LlmPlugin;

impl Plugin for LlmPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(LlmRequestQueue::default())
            .insert_resource(LlmResponseQueue::default())
            .insert_resource(LlmLoadingState::default())
            .insert_resource(OllamaConfig::default())
            .insert_resource(OllamaTaskState::default())
            .insert_resource(EventHistory::default())
            .add_systems(Update, (
                process_ollama_requests,
                process_llm_responses.after(process_ollama_requests),
            ));
    }
}

#[derive(Resource, Clone)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
}

impl OllamaConfig {
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3.2".to_string()),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[derive(Resource, Default)]
pub struct LlmLoadingState {
    pub is_loading: bool,
    pub loading_text: String,
}

#[derive(Resource)]
pub struct OllamaTaskState {
    pub runtime: Arc<Runtime>,
    pub task: Option<tokio::task::JoinHandle<Result<LlmResponse, String>>>,
    pub pending_request: Option<LlmRequest>,
}

impl Default for OllamaTaskState {
    fn default() -> Self {
        Self {
            runtime: Arc::new(Runtime::new().expect("Failed to create tokio runtime")),
            task: None,
            pending_request: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameNarrativeContext {
    pub sector_name: String,
    pub sector_type: SectorType,
    pub danger_level: u32,
    pub fuel: f32,
    pub scrap: u32,
    pub distance_traveled: u32,
    pub recent_event_titles: Vec<String>, // Last 5 event titles
    pub total_events_seen: u32,
}

#[derive(Clone, Debug)]
pub enum LlmRequest {
    GenerateEvent {
        context: GameNarrativeContext,
    },
}

#[derive(Clone, Debug)]
pub enum LlmResponse {
    GeneratedEvent {
        event: GeneratedEvent,
    },
}

#[derive(Clone, Debug)]
pub struct GeneratedEvent {
    pub title: String,
    pub stages: Vec<EventStage>, // Max 3 stages: start, middle, end
}

#[derive(Clone, Debug)]
pub struct EventStage {
    pub stage_number: u32, // 0=start, 1=middle, 2=end
    pub description: String, // What happens at this stage
    pub choices: Vec<GeneratedChoice>,
}

#[derive(Clone, Debug)]
pub struct GeneratedChoice {
    pub text: String,
    pub outcome: ChoiceOutcome,
    pub next_stage: Option<u32>, // Which stage this leads to (None = event ends)
}

#[derive(Clone, Debug)]
pub struct ChoiceOutcome {
    pub outcome_text: String, // Narrative description of what happens
    pub fuel_delta: RewardRange, // Can be fixed or random range
    pub scrap_delta: RewardRange,
    pub hull_delta: RewardRange,
}

#[derive(Clone, Debug)]
pub enum RewardRange {
    Fixed(f32), // Fixed value
    Random { min: f32, max: f32 }, // Random range, e.g., 4-10
}

#[derive(Resource, Default)]
pub struct LlmRequestQueue(pub Vec<LlmRequest>);

#[derive(Resource, Default)]
pub struct LlmResponseQueue(pub Vec<LlmResponse>);

#[derive(Resource, Default)]
pub struct EventHistory {
    pub recent_events: Vec<String>, // Titles of recent events (last 5)
    pub total_events: u32,
}

impl EventHistory {
    pub fn add_event(&mut self, title: String) {
        self.recent_events.push(title);
        self.total_events += 1;
        // Keep only last 5 events
        if self.recent_events.len() > 5 {
            self.recent_events.remove(0);
        }
    }
}

fn process_ollama_requests(
    mut request_queue: ResMut<LlmRequestQueue>,
    mut loading_state: ResMut<LlmLoadingState>,
    mut task_state: ResMut<OllamaTaskState>,
    mut response_queue: ResMut<LlmResponseQueue>,
    config: Res<OllamaConfig>,
) {
    // Check if there's an active task
    let task_finished = task_state.task.as_ref().map(|t| t.is_finished()).unwrap_or(false);
    if task_finished {
        if let Some(task) = task_state.task.take() {
            // Task completed, get the result
            let runtime = task_state.runtime.clone();
            let result = runtime.block_on(async {
                task.await
            });
            
            match result {
                Ok(Ok(response)) => {
                    println!("[LLM] Received response from Ollama");
                    response_queue.0.push(response);
                    loading_state.is_loading = false;
                    task_state.pending_request = None;
                }
                Ok(Err(e)) => {
                    eprintln!("[LLM] Ollama request failed: {}. Using fallback event.", e);
                    loading_state.is_loading = false;
                    if let Some(request) = task_state.pending_request.take() {
                        let fallback_response = handle_llm_request_fallback(request);
                        response_queue.0.push(fallback_response);
                    }
                }
                Err(e) => {
                    eprintln!("[LLM] Task join error: {}. Using fallback event.", e);
                    loading_state.is_loading = false;
                    if let Some(request) = task_state.pending_request.take() {
                        let fallback_response = handle_llm_request_fallback(request);
                        response_queue.0.push(fallback_response);
                    }
                }
            }
        }
        return;
    }
    
    // Process new requests
    if let Some(request) = request_queue.0.pop() {
        println!("[LLM] Processing Ollama request...");
        loading_state.is_loading = true;
        loading_state.loading_text = "Generating event...".to_string();
        
        let config_clone = OllamaConfig {
            url: config.url.clone(),
            model: config.model.clone(),
        };
        let request_clone = request.clone();
        let runtime = task_state.runtime.clone();
        
        // Store request for fallback
        task_state.pending_request = Some(request.clone());
        
        // Spawn async task
        let handle = runtime.spawn(async move {
            handle_llm_request_ollama(request_clone, config_clone.url, config_clone.model).await
        });
        
        task_state.task = Some(handle);
    }
}

fn process_llm_responses(
    mut response_queue: ResMut<LlmResponseQueue>,
    mut event_writer: MessageWriter<GameEvent>,
    mut active_event: ResMut<crate::events::ActiveEvent>,
    mut loading_state: ResMut<LlmLoadingState>,
    mut event_history: ResMut<EventHistory>,
    mut game_data: ResMut<crate::game::GameData>,
) {
    for response in response_queue.0.drain(..) {
        match response {
            LlmResponse::GeneratedEvent { event } => {
                println!("[LLM] ========================================");
                println!("[LLM] Received generated event: \"{}\"", event.title);
                println!("[LLM] Number of stages: {}", event.stages.len());
                for stage in &event.stages {
                    println!("[LLM]   Stage {}: {} choices", stage.stage_number, stage.choices.len());
                    for (i, choice) in stage.choices.iter().enumerate() {
                        println!("[LLM]     Choice {}: {} -> stage {:?}", 
                            i + 1, choice.text, choice.next_stage);
                    }
                }
                println!("[LLM] ========================================");
                
                let game_event = convert_generated_to_game_event(event);
                println!("[LLM] Setting active event: \"{}\"", game_event.title);
                active_event.event = Some(game_event.clone());
                active_event.state = crate::events::EventState::Initial;
                active_event.outcome_history.clear();
                event_writer.write(game_event.clone());
                
                // Track event in history
                event_history.add_event(game_event.title.clone());
                
                loading_state.is_loading = false;
                println!("[LLM] Event written and active_event updated");
            }
        }
    }
}

async fn handle_llm_request_ollama(
    request: LlmRequest,
    ollama_url: String,
    model_name: String,
) -> Result<LlmResponse, String> {
    match request {
        LlmRequest::GenerateEvent { context } => {
            let prompt = build_event_generation_prompt(&context);
            println!("[LLM] Sending request to Ollama: {}", ollama_url);
            println!("[LLM] Model: {}", model_name);
            println!("[LLM] Prompt ({} chars):\n{}", prompt.len(), prompt);
            
            let client = reqwest::Client::new();
            let url = format!("{}/api/generate", ollama_url);
            
            let request_body = json!({
                "model": model_name,
                "prompt": prompt,
                "stream": false,
                "options": {
                    "num_predict": 300,
                    "temperature": 0.9,
                    "top_p": 0.95,
                    "repeat_penalty": 1.1,
                }
            });
            
            let start = std::time::Instant::now();
            let response = client
                .post(&url)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| format!("HTTP error: {}", e))?;
            
            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                return Err(format!("API error {}: {}", status, text));
            }
            
            let result: Value = response
                .json()
                .await
                .map_err(|e| format!("JSON parse error: {}", e))?;
            
            let elapsed = start.elapsed();
            println!("[LLM] Response received in {:?}", elapsed);
            
            let generated_text = result.get("response")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'response' field in Ollama response".to_string())?;
            
            println!("[LLM] Generated text:\n{}", generated_text);
            
            parse_llm_response(generated_text)
        }
    }
}

fn convert_generated_to_game_event(generated: GeneratedEvent) -> GameEvent {
    // Convert stages to GameEventStage format
    let game_stages: Vec<crate::events::GameEventStage> = generated.stages.iter().map(|stage| {
        let stage_choices: Vec<crate::events::StageChoice> = stage.choices.iter().map(|c| {
            crate::events::StageChoice {
                text: c.text.clone(),
                outcome_text: c.outcome.outcome_text.clone(),
                fuel_delta: match &c.outcome.fuel_delta {
                    RewardRange::Fixed(v) => crate::events::RewardRange::Fixed(*v),
                    RewardRange::Random { min, max } => crate::events::RewardRange::Random { min: *min, max: *max },
                },
                scrap_delta: match &c.outcome.scrap_delta {
                    RewardRange::Fixed(v) => crate::events::RewardRange::Fixed(*v),
                    RewardRange::Random { min, max } => crate::events::RewardRange::Random { min: *min, max: *max },
                },
                hull_delta: match &c.outcome.hull_delta {
                    RewardRange::Fixed(v) => crate::events::RewardRange::Fixed(*v),
                    RewardRange::Random { min, max } => crate::events::RewardRange::Random { min: *min, max: *max },
                },
                next_stage: c.next_stage,
            }
        }).collect();
        
        crate::events::GameEventStage {
            stage_number: stage.stage_number,
            description: stage.description.clone(),
            choices: stage_choices,
        }
    }).collect();
    
    // Get initial stage (stage 0) for current choices
    let initial_stage = generated.stages.first()
        .expect("Event must have at least stage 0");
    
    // Convert initial stage choices to EventChoice format for display
    let initial_choices: Vec<crate::events::EventChoice> = initial_stage.choices.iter().map(|c| {
        // Create a placeholder outcome - actual outcome will be processed when choice is made
        crate::events::EventChoice {
            text: c.text.clone(),
            outcome: crate::events::EventOutcome::Continue, // Placeholder
            requirements: vec![],
        }
    }).collect();
    
    GameEvent {
        title: generated.title,
        description: initial_stage.description.clone(), // Start with stage 0 description
        _event_type: GameEventType::Story,
        choices: initial_choices,
        _faction: None,
        current_stage: 0,
        stages: game_stages,
    }
}

// Removed: build_outcome_generation_prompt - no longer needed with pre-generated events
// Removed: parse_outcome_response - no longer needed with pre-generated events

pub fn build_event_generation_prompt(context: &GameNarrativeContext) -> String {
    let sector_type_desc = match context.sector_type {
        SectorType::Empty => "Empty space with nothing of particular interest",
        SectorType::Nebula => "A colorful nebula that interferes with sensors but provides cover",
        SectorType::AsteroidField => "A dense field of asteroids rich in minerals",
        SectorType::Station => "A space station offering services to travelers",
        SectorType::Distress => "A distress beacon emanates from this location",
        SectorType::Combat => "Hostile ships patrol this area",
        SectorType::Anomaly => "Strange energy readings suggest something unusual here",
        SectorType::DarkRift => "A fragment of the mysterious Dark Rift - dangerous but potentially rewarding",
        SectorType::CelestialSite => "Ancient ruins left by the Celestials, humming with residual power",
        SectorType::AetheriumField => "Rare Aetherium crystals float in the cosmic void here",
    };
    
    let recent_events_text = if context.recent_event_titles.is_empty() {
        "None yet - this is the first event.".to_string()
    } else {
        format!("Recent events: {}", context.recent_event_titles.join(", "))
    };
    
    format!(
        r#"You are a creative writer for a space exploration game. Generate a COMPLETE multi-stage event with ALL outcomes pre-determined. The event has a maximum of 3 stages (start, middle, end).

GAME CONTEXT:
- Current Sector: {} ({})
- Sector Description: {}
- Danger Level: {} (0=safe, 5=extremely dangerous, 8+=deadly)
- Distance Traveled: {} sectors
- Current Fuel: {:.0}
- Current Scrap: {}
- Total Events Encountered: {}
- {}

CRITICAL REQUIREMENTS:
- Generate a COMPLETE event tree with ALL stages and outcomes pre-determined
- Maximum 3 stages: stage 0 (start), stage 1 (middle, optional), stage 2 (end, optional)
- Each choice must specify which stage it leads to (or null to end event)
- Each choice must have a pre-determined outcome with narrative text
- Rewards can be fixed values OR random ranges (e.g., "4-10" for fuel)
- Create a COMPLETELY UNIQUE event - never repeat previous events
- Vary the narrative style: sometimes mysterious, sometimes action-packed, sometimes contemplative, sometimes humorous
- Make titles creative and specific (avoid generic words like "Encounter", "Event")

OUTPUT FORMAT (JSON only, no markdown code blocks, no explanations):
{{
  "title": "Creative and specific event title",
  "stages": [
    {{
      "stage_number": 0,
      "description": "Initial event description - 2-4 sentences setting the scene",
      "choices": [
        {{
          "text": "Choice 1 text",
          "outcome": {{
            "outcome_text": "What happens when player chooses this - 2-4 sentences",
            "fuel_delta": {{"type": "fixed", "value": -2.0}},
            "scrap_delta": {{"type": "random", "min": 5, "max": 10}},
            "hull_delta": {{"type": "fixed", "value": 0}}
          }},
          "next_stage": 1
        }},
        {{
          "text": "Choice 2 text",
          "outcome": {{
            "outcome_text": "What happens - 2-4 sentences",
            "fuel_delta": {{"type": "fixed", "value": 0}},
            "scrap_delta": {{"type": "fixed", "value": -3}},
            "hull_delta": {{"type": "fixed", "value": 0}}
          }},
          "next_stage": null
        }}
      ]
    }},
    {{
      "stage_number": 1,
      "description": "What happens in the middle stage - 2-4 sentences",
      "choices": [
        {{
          "text": "Middle stage choice",
          "outcome": {{
            "outcome_text": "Outcome narrative - 2-4 sentences",
            "fuel_delta": {{"type": "random", "min": 4, "max": 10}},
            "scrap_delta": {{"type": "fixed", "value": 5}},
            "hull_delta": {{"type": "fixed", "value": -1}}
          }},
          "next_stage": 2
        }}
      ]
    }},
    {{
      "stage_number": 2,
      "description": "Final stage description - 2-4 sentences",
      "choices": [
        {{
          "text": "Final choice",
          "outcome": {{
            "outcome_text": "Final outcome - 2-4 sentences with conclusion",
            "fuel_delta": {{"type": "fixed", "value": 2.0}},
            "scrap_delta": {{"type": "random", "min": 8, "max": 15}},
            "hull_delta": {{"type": "fixed", "value": 0}}
          }},
          "next_stage": null
        }}
      ]
    }}
  ]
}}

RULES:
- Provide 1-3 stages (stage 0 is required, stages 1-2 are optional)
- Each stage has 2-4 choices
- Each choice has an outcome with narrative text and stat changes
- next_stage: null means event ends, number means go to that stage
- Rewards: Use "fixed" for exact values, "random" for ranges (min/max)
- fuel_delta: f32 (can be decimal)
- scrap_delta: integer (whole numbers only)
- hull_delta: integer (whole numbers only)
- Use numbers only: 5 (NOT +5), -5 (negative is fine)
- Make the event feel complete and satisfying

Generate the JSON now:"#,
        context.sector_name,
        format!("{:?}", context.sector_type),
        sector_type_desc,
        context.danger_level,
        context.distance_traveled,
        context.fuel,
        context.scrap,
        context.total_events_seen,
        recent_events_text,
    )
}

fn parse_llm_response(text: &str) -> Result<LlmResponse, String> {
    let json_text = if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            &text[start..=end]
        } else {
            eprintln!("[LLM] No closing brace found in response");
            eprintln!("[LLM] Raw generated text:\n{}", text);
            return Err("No closing brace found".to_string());
        }
    } else {
        eprintln!("[LLM] No JSON found in response");
        eprintln!("[LLM] Raw generated text:\n{}", text);
        return Err("No JSON found in response".to_string());
    };
    
    // Clean up invalid JSON syntax
    let cleaned_json = json_text
        .replace(r#": +"#, r#": "#)
        .replace(r#":+"#, r#": "#)
        .replace(r#", +"#, r#", "#)
        .replace(r#" +"#, r#" "#)
        .lines()
        .map(|line| {
            line.replace(r#": +"#, r#": "#)
                .replace(r#":+"#, r#": "#)
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    println!("[LLM] ========================================");
    println!("[LLM] EXTRACTED JSON:");
    println!("[LLM] ========================================");
    println!("{}", cleaned_json);
    println!("[LLM] ========================================");
    
    let value: Value = serde_json::from_str(&cleaned_json)
        .map_err(|e| {
            eprintln!("[LLM] JSON parse error: {}", e);
            eprintln!("[LLM] Raw JSON text:\n{}", cleaned_json);
            format!("Failed to parse JSON: {}", e)
        })?;
    
    let title = value["title"].as_str()
        .ok_or("Missing 'title' field")?
        .to_string();
    
    let stages_array = value["stages"].as_array()
        .ok_or("Missing or invalid 'stages' array")?;
    
    let stages: Result<Vec<EventStage>, String> = stages_array.iter()
        .map(parse_stage)
        .collect();
    
    let stages = stages?;
    
    // Validate: must have at least stage 0, max 3 stages
    if stages.is_empty() {
        return Err("Event must have at least one stage".to_string());
    }
    if stages.len() > 3 {
        return Err(format!("Event has too many stages: {} (max 3)", stages.len()));
    }
    
    Ok(LlmResponse::GeneratedEvent {
        event: GeneratedEvent {
            title,
            stages,
        },
    })
}

fn parse_stage(stage_value: &Value) -> Result<EventStage, String> {
    let stage_number = stage_value["stage_number"].as_u64()
        .ok_or("Missing or invalid 'stage_number'")? as u32;
    
    let description = stage_value["description"].as_str()
        .ok_or("Missing 'description' in stage")?
        .to_string();
    
    let choices_array = stage_value["choices"].as_array()
        .ok_or("Missing or invalid 'choices' array in stage")?;
    
    let choices: Result<Vec<GeneratedChoice>, String> = choices_array.iter()
        .map(|c| parse_choice(c))
        .collect();
    
    Ok(EventStage {
        stage_number,
        description,
        choices: choices?,
    })
}

fn parse_choice(choice_value: &Value) -> Result<GeneratedChoice, String> {
    let text = choice_value["text"].as_str()
        .ok_or("Missing 'text' in choice")?
        .to_string();
    
    let outcome_value = choice_value.get("outcome")
        .ok_or("Missing 'outcome' in choice")?;
    
    let outcome_text = outcome_value["outcome_text"].as_str()
        .ok_or("Missing 'outcome_text' in outcome")?
        .to_string();
    
    let fuel_delta = parse_reward_range(outcome_value.get("fuel_delta"))?;
    let scrap_delta = parse_reward_range(outcome_value.get("scrap_delta"))?;
    let hull_delta = parse_reward_range(outcome_value.get("hull_delta"))?;
    
    // next_stage is at the choice level, not outcome level
    let next_stage = choice_value["next_stage"].as_u64()
        .map(|n| n as u32);
    
    Ok(GeneratedChoice {
        text,
        outcome: ChoiceOutcome {
            outcome_text,
            fuel_delta,
            scrap_delta,
            hull_delta,
        },
        next_stage,
    })
}

fn parse_reward_range(value: Option<&Value>) -> Result<RewardRange, String> {
    let val = value.ok_or("Missing reward value")?;
    
    let reward_type = val["type"].as_str()
        .ok_or("Missing 'type' in reward")?;
    
    match reward_type {
        "fixed" => {
            let v = val["value"].as_f64()
                .ok_or("Missing 'value' in fixed reward")? as f32;
            Ok(RewardRange::Fixed(v))
        }
        "random" => {
            let min = val["min"].as_f64()
                .ok_or("Missing 'min' in random reward")? as f32;
            let max = val["max"].as_f64()
                .ok_or("Missing 'max' in random reward")? as f32;
            Ok(RewardRange::Random { min, max })
        }
        _ => Err(format!("Invalid reward type: {}", reward_type))
    }
}

pub fn handle_llm_request_fallback(request: LlmRequest) -> LlmResponse {
    match request {
        LlmRequest::GenerateEvent { context } => {
            // Create a simple single-stage fallback event
            let fallback_stage = EventStage {
                stage_number: 0,
                description: format!("You arrive at {}. What will you do?", context.sector_name),
                choices: vec![
                    GeneratedChoice {
                        text: "Explore".to_string(),
                        outcome: ChoiceOutcome {
                            outcome_text: "You explore the area and find some useful scrap.".to_string(),
                            fuel_delta: RewardRange::Fixed(0.0),
                            scrap_delta: RewardRange::Fixed(5.0),
                            hull_delta: RewardRange::Fixed(0.0),
                        },
                        next_stage: None,
                    },
                    GeneratedChoice {
                        text: "Move on".to_string(),
                        outcome: ChoiceOutcome {
                            outcome_text: "You continue your journey.".to_string(),
                            fuel_delta: RewardRange::Fixed(0.0),
                            scrap_delta: RewardRange::Fixed(0.0),
                            hull_delta: RewardRange::Fixed(0.0),
                        },
                        next_stage: None,
                    },
                ],
            };
            
            LlmResponse::GeneratedEvent {
                event: GeneratedEvent {
                    title: format!("Encounter in {}", context.sector_name),
                    stages: vec![fallback_stage],
                },
            }
        }
    }
}
