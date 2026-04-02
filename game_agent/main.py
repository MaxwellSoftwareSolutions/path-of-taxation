#!/usr/bin/env python3
"""CLI entry point for the Path of Taxation game testing agent.

Usage examples::

    python -m game_agent --mode observe
    python -m game_agent --mode auto --max-loops 50
    python -m game_agent --mode record --session test1
    python -m game_agent --mode observe --dry-run
    python -m game_agent --mode observe --no-model
"""

from __future__ import annotations

import argparse
import sys


def build_parser() -> argparse.ArgumentParser:
    """Build and return the argument parser."""
    parser = argparse.ArgumentParser(
        prog="game_agent",
        description="AI game testing agent for Path of Taxation.",
    )
    parser.add_argument(
        "--mode",
        choices=("observe", "assist", "auto", "record"),
        default="observe",
        help="Agent operating mode (default: observe).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        default=False,
        help="Log actions but do not execute them.",
    )
    parser.add_argument(
        "--max-loops",
        type=int,
        default=None,
        help="Override maximum number of loop iterations.",
    )
    parser.add_argument(
        "--session",
        type=str,
        default=None,
        help="Session name for 'record' mode.",
    )
    parser.add_argument(
        "--no-model",
        action="store_true",
        default=False,
        help="Skip VLM loading; use dummy analysis for pipeline testing.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        default=False,
        help="Enable DEBUG-level logging.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    """Parse arguments, configure logging, and run the game loop."""
    parser = build_parser()
    args = parser.parse_args(argv)

    # -- Logging -----------------------------------------------------------
    from game_agent.logger import setup_logging  # noqa: PLC0415

    setup_logging(verbose=args.verbose)

    # -- Resolve config overrides ------------------------------------------
    from game_agent import config  # noqa: PLC0415

    max_loops = args.max_loops if args.max_loops is not None else config.MAX_LOOPS
    dry_run = args.dry_run or config.DRY_RUN

    # -- Print banner ------------------------------------------------------
    import logging  # noqa: PLC0415

    log = logging.getLogger("game_agent.main")
    log.info("Path of Taxation -- Game Testing Agent v0.1.0")
    log.info(
        "mode=%s  dry_run=%s  max_loops=%d  no_model=%s",
        args.mode,
        dry_run,
        max_loops,
        args.no_model,
    )

    # -- Construct and run the loop ----------------------------------------
    from game_agent.game_loop import GameLoop  # noqa: PLC0415

    loop = GameLoop(
        mode=args.mode,
        dry_run=dry_run,
        max_loops=max_loops,
        session_name=args.session,
        no_model=args.no_model,
    )
    loop.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
