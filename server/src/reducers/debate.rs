use spacetimedb::{reducer, ReducerContext, Table};

use crate::tables::player::player;
use crate::tables::run::run;
use crate::tables::debate::{debate_reward, debate_session, DebateReward, DebateSession};

/// Start a Debate Club session against an NPC opponent.
#[reducer]
pub fn start_debate(ctx: &ReducerContext, opponent_key: String) -> Result<(), String> {
    let sender = ctx.sender();

    // Validate the player exists.
    ctx.db
        .player()
        .identity()
        .find(sender)
        .ok_or("Player not found.")?;

    // Check no active debate session exists.
    let has_active = ctx
        .db
        .debate_session()
        .iter()
        .any(|s| s.owner == sender && s.status == "active");
    if has_active {
        return Err("You already have an active debate session.".into());
    }

    // Check the player does not have an active run.
    let has_active_run = ctx
        .db
        .run()
        .iter()
        .any(|r| r.owner == sender && r.status == "active");
    if has_active_run {
        return Err("Cannot start a debate during an active run.".into());
    }

    // Create the session with a starting hand and deck.
    // Stub: use a simple default deck stored as comma-separated strings.
    let default_hand =
        "ad_hominem,straw_man,appeal_to_authority,burden_of_proof,moving_the_goalposts";
    let default_draw = "objection,counter_argument,evidence,rhetoric_flourish,closing_statement";

    ctx.db.debate_session().insert(DebateSession {
        id: 0,
        owner: sender,
        opponent_key,
        status: "active".into(),
        current_turn: 1,
        player_credibility: 100,
        opponent_credibility: 100,
        rhetoric_points: 3,
        max_rhetoric_points: 3,
        hand_json: default_hand.into(),
        draw_pile_json: default_draw.into(),
        discard_pile_json: String::new(),
        started_at: ctx.timestamp,
    });

    log::info!("Debate session started for {:?}", sender);
    Ok(())
}

/// Play a card from the player's hand during a debate.
#[reducer]
pub fn play_card(
    ctx: &ReducerContext,
    session_id: u64,
    card_key: String,
) -> Result<(), String> {
    let sender = ctx.sender();

    let mut session = ctx
        .db
        .debate_session()
        .id()
        .find(session_id)
        .ok_or("Debate session not found.")?;
    if session.owner != sender {
        return Err("This is not your debate session.".into());
    }
    if session.status != "active" {
        return Err("Debate session is not active.".into());
    }

    // Parse the hand (comma-separated card keys).
    let mut hand: Vec<String> = if session.hand_json.is_empty() {
        Vec::new()
    } else {
        session.hand_json.split(',').map(|s| s.to_string()).collect()
    };
    let mut discard: Vec<String> = if session.discard_pile_json.is_empty() {
        Vec::new()
    } else {
        session
            .discard_pile_json
            .split(',')
            .map(|s| s.to_string())
            .collect()
    };

    // Find and remove the card from the hand.
    let card_index = hand
        .iter()
        .position(|c| c == &card_key)
        .ok_or(format!("Card '{}' not in hand.", card_key))?;
    hand.remove(card_index);

    // Check rhetoric points cost (stub: all cards cost 1).
    let card_cost: i64 = 1;
    if session.rhetoric_points < card_cost {
        return Err("Not enough Rhetoric Points.".into());
    }
    session.rhetoric_points -= card_cost;

    // Apply card effect (stub: deal 10 damage to opponent credibility).
    // TODO: look up card definitions and apply proper effects.
    let card_damage: i64 = 10;
    session.opponent_credibility -= card_damage;

    // Move the card to discard.
    discard.push(card_key);

    // Check win condition.
    if session.opponent_credibility <= 0 {
        session.status = "won".into();
        session.opponent_credibility = 0;

        // Award a debate reward.
        ctx.db.debate_reward().insert(DebateReward {
            id: 0,
            owner: sender,
            session_id: session.id,
            modifier_key: "bonus_damage_10pct".into(),
            is_available: true,
            earned_at: ctx.timestamp,
        });

        log::info!("Debate won by {:?}!", sender);
    }

    // Update hand and discard as comma-separated strings.
    session.hand_json = hand.join(",");
    session.discard_pile_json = discard.join(",");

    ctx.db.debate_session().id().update(session);
    Ok(())
}

/// End the player's turn in a debate. The opponent takes their turn (stub),
/// then a new turn begins with refreshed Rhetoric Points.
#[reducer]
pub fn end_debate_turn(ctx: &ReducerContext, session_id: u64) -> Result<(), String> {
    let sender = ctx.sender();

    let mut session = ctx
        .db
        .debate_session()
        .id()
        .find(session_id)
        .ok_or("Debate session not found.")?;
    if session.owner != sender {
        return Err("This is not your debate session.".into());
    }
    if session.status != "active" {
        return Err("Debate session is not active.".into());
    }

    // Opponent's turn (stub: deal 8 damage to player credibility).
    // TODO: implement opponent AI based on opponent_key card strategies.
    let opponent_damage: i64 = 8;
    session.player_credibility -= opponent_damage;

    // Check loss condition.
    if session.player_credibility <= 0 {
        session.status = "lost".into();
        session.player_credibility = 0;
        ctx.db.debate_session().id().update(session);
        log::info!("Debate lost by {:?}", sender);
        return Ok(());
    }

    // Advance to next turn.
    session.current_turn += 1;

    // Increase max rhetoric points each turn (like Slay the Spire energy).
    session.max_rhetoric_points = (session.max_rhetoric_points + 1).min(10);
    session.rhetoric_points = session.max_rhetoric_points;

    // Draw a card from the draw pile.
    let mut draw_pile: Vec<String> = if session.draw_pile_json.is_empty() {
        Vec::new()
    } else {
        session
            .draw_pile_json
            .split(',')
            .map(|s| s.to_string())
            .collect()
    };
    let mut hand: Vec<String> = if session.hand_json.is_empty() {
        Vec::new()
    } else {
        session.hand_json.split(',').map(|s| s.to_string()).collect()
    };

    if let Some(card) = draw_pile.pop() {
        hand.push(card);
    } else {
        // Shuffle discard into draw pile.
        let discard: Vec<String> = if session.discard_pile_json.is_empty() {
            Vec::new()
        } else {
            session
                .discard_pile_json
                .split(',')
                .map(|s| s.to_string())
                .collect()
        };
        draw_pile = discard;
        session.discard_pile_json = String::new();
        // Draw one card from the reshuffled pile.
        if let Some(card) = draw_pile.pop() {
            hand.push(card);
        }
    }

    session.hand_json = hand.join(",");
    session.draw_pile_json = draw_pile.join(",");

    ctx.db.debate_session().id().update(session);
    Ok(())
}
