#!/usr/bin/env python3
"""Quick visual quality check on a game screenshot.

Usage:
    python critic_cli.py screenshot.png                    # Tier 1+2
    python critic_cli.py screenshot.png --tier 1           # fast checks only
    python critic_cli.py screenshot.png --tier 3           # all tiers (loads VLM)
    python critic_cli.py screenshot.png --reference ./ref/  # CLIP similarity
    python critic_cli.py screenshot.png --json             # raw JSON output

Exit code: 0 = PASS, 1 = FAIL
"""

from __future__ import annotations

import argparse
import json
import logging
import sys
from pathlib import Path


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="critic_cli",
        description="Visual quality critic for game screenshots.",
    )
    parser.add_argument(
        "screenshot",
        type=Path,
        help="Path to the screenshot to evaluate.",
    )
    parser.add_argument(
        "--tier",
        type=int,
        default=2,
        choices=[1, 2, 3],
        help="Max evaluation tier (1=programmatic, 2=+neural, 3=+VLM). Default: 2.",
    )
    parser.add_argument(
        "--reference",
        type=Path,
        default=None,
        help="Directory of reference images for CLIP similarity.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output raw JSON instead of human-readable summary.",
    )
    parser.add_argument(
        "--all-tiers",
        action="store_true",
        help="Run all tiers even if earlier ones fail.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable DEBUG logging.",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(levelname)s %(name)s: %(message)s",
    )

    if not args.screenshot.exists():
        print(f"ERROR: File not found: {args.screenshot}", file=sys.stderr)
        return 2

    # Tier 1 only -- no heavy imports needed.
    if args.tier == 1:
        from visual_critic import run_tier1  # noqa: PLC0415

        result = run_tier1(args.screenshot)
        if args.json:
            print(json.dumps(result, indent=2))
        else:
            print("Tier 1 (programmatic) results:")
            for k, v in sorted(result.items()):
                print(f"  {k}: {v}")
            print(f"\n  Result: {'PASS' if result['tier1_pass'] else 'FAIL'}")
        return 0 if result["tier1_pass"] else 1

    # Tier 2+ uses the full pipeline.
    from visual_critic import evaluate_screenshot  # noqa: PLC0415

    vision_agent = None
    if args.tier >= 3:
        from game_agent.config import (  # noqa: PLC0415
            MAX_NEW_TOKENS,
            MODEL_ID,
            TEMPERATURE,
        )
        from game_agent.vision_agent import VisionAgent  # noqa: PLC0415

        vision_agent = VisionAgent(
            MODEL_ID,
            max_new_tokens=MAX_NEW_TOKENS,
            temperature=TEMPERATURE,
        )

    report = evaluate_screenshot(
        args.screenshot,
        vision_agent=vision_agent,
        reference_dir=args.reference,
        run_all_tiers=args.all_tiers,
    )

    if args.json:
        print(json.dumps(report.to_dict(), indent=2))
    else:
        print(report.summary())

    return 0 if report.overall_pass else 1


if __name__ == "__main__":
    sys.exit(main())
