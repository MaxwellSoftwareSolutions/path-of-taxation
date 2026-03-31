# Implementation Roadmap

## Immediate Goal

Reach a playable internal prototype as fast as possible.

The prototype does not need content breadth. It needs one repeatable loop that proves:

- movement feels responsive
- enemies create pressure
- upgrade rewards are understandable
- one boss changes player behavior
- the hub transition is worth repeating

## Milestone 1: Movement Room

Deliverables:

- player movement
- camera setup
- one room shell
- screen-state transitions between hub and run

Exit criteria:

- player can enter a run and move reliably
- returning to the hub works without leaks or broken state

## Milestone 2: First Combat

Deliverables:

- one basic enemy
- player primary attack
- hit detection
- health and damage model
- death and retry flow

Exit criteria:

- one room can be won or lost
- combat impact is readable

## Milestone 3: Reward Loop

Deliverables:

- post-room reward selection
- 6 to 8 test upgrades
- simple run currency

Exit criteria:

- two runs can diverge meaningfully from the same start

## Milestone 4: Boss Slice

Deliverables:

- boss arena
- three-phase boss script
- one altered-combat-rule phase change

Exit criteria:

- the run ends in a distinct climax instead of another room clear

## Milestone 5: Debate Club

Deliverables:

- turn-based minigame screen
- 5 player cards
- 3 NPC intents
- reward injection into next run

Exit criteria:

- players understand why the minigame exists
- it creates anticipation for the next attempt

## Milestone 6: Polish The Loop

Deliverables:

- basic VFX and juice
- stronger UI prompts
- first pass on satirical item/reward text
- balancing of room count and pacing

Exit criteria:

- a full run feels worth replaying immediately

