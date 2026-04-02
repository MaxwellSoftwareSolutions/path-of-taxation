"""Visual quality critic for terrain and game scene evaluation.

Runs a three-tier quality assessment pipeline:
  Tier 1: Fast programmatic checks (seams, repetition, color, lines)
  Tier 2: Neural aesthetic scoring (NIMA, BRISQUE via pyiqa)
  Tier 3: VLM deep critique (Qwen2.5-VL checklist evaluation)

Usage:
    from visual_critic import evaluate_screenshot

    report = evaluate_screenshot("screenshot.png")
    print(report.summary())
"""

from __future__ import annotations

import json
import logging
from dataclasses import asdict, dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Quality report
# ---------------------------------------------------------------------------


@dataclass
class QualityReport:
    """Structured quality assessment result."""

    screenshot: str
    tier1_passed: bool = True
    tier2_passed: bool = True
    tier3_passed: bool = True
    overall_pass: bool = True

    # Tier 1: programmatic
    seam_score: float = 0.0
    repetition_score: float = 0.0
    contrast_ratio: float = 0.0
    long_straight_lines: int = 0
    detail_consistency: float = 0.0

    # Tier 2: neural
    nima_score: float | None = None
    brisque_score: float | None = None
    clip_reference_similarity: float | None = None

    # Tier 3: VLM
    vlm_overall_score: int | None = None
    vlm_worst_problems: list[str] = field(default_factory=list)
    vlm_actionable_fixes: list[str] = field(default_factory=list)

    # Meta
    is_regression: bool = False
    regression_details: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)

    def summary(self) -> str:
        lines = [f"Visual Quality Report: {self.screenshot}"]
        lines.append(f"  Overall: {'PASS' if self.overall_pass else 'FAIL'}")
        lines.append(f"  Tier 1 (programmatic): {'PASS' if self.tier1_passed else 'FAIL'}")
        lines.append(f"    Seam score: {self.seam_score:.3f} (< 0.3 = ok)")
        lines.append(f"    Repetition: {self.repetition_score:.3f} (< 0.5 = ok)")
        lines.append(f"    Contrast ratio: {self.contrast_ratio:.1f}")
        lines.append(f"    Straight lines: {self.long_straight_lines}")
        lines.append(f"    Detail consistency: {self.detail_consistency:.3f}")
        if self.nima_score is not None:
            lines.append(
                f"  Tier 2 (neural): {'PASS' if self.tier2_passed else 'FAIL'}"
            )
            lines.append(f"    NIMA aesthetic: {self.nima_score:.3f}")
            if self.brisque_score is not None:
                lines.append(f"    BRISQUE quality: {self.brisque_score:.3f}")
            if self.clip_reference_similarity is not None:
                lines.append(
                    f"    CLIP ref similarity: {self.clip_reference_similarity:.3f}"
                )
        if self.vlm_overall_score is not None:
            lines.append(
                f"  Tier 3 (VLM): {'PASS' if self.tier3_passed else 'FAIL'}"
            )
            lines.append(f"    VLM score: {self.vlm_overall_score}/10")
            if self.vlm_worst_problems:
                lines.append(
                    f"    Worst problems: {', '.join(self.vlm_worst_problems[:3])}"
                )
            if self.vlm_actionable_fixes:
                lines.append(
                    f"    Suggested fixes: {', '.join(self.vlm_actionable_fixes[:3])}"
                )
        if self.is_regression:
            lines.append(
                f"  REGRESSION DETECTED: {'; '.join(self.regression_details)}"
            )
        return "\n".join(lines)


# ---------------------------------------------------------------------------
# Tier 1: Programmatic checks (no GPU needed, < 1 second)
# ---------------------------------------------------------------------------


def check_seams(img_gray: np.ndarray, expected_tile_size: int = 64) -> dict:
    """Detect tile seams via Canny + Hough line transform."""
    import cv2  # noqa: PLC0415

    edges = cv2.Canny(img_gray, 50, 150)
    lines = cv2.HoughLinesP(
        edges, 1, np.pi / 180, threshold=100, minLineLength=100, maxLineGap=10
    )
    if lines is None:
        return {"seam_score": 0.0, "grid_aligned": 0, "total_edges": 0}

    grid_aligned = 0
    for line in lines:
        x1, y1, x2, y2 = line[0]
        if y1 % expected_tile_size < 3 or x1 % expected_tile_size < 3:
            grid_aligned += 1

    return {
        "seam_score": grid_aligned / max(len(lines), 1),
        "grid_aligned": grid_aligned,
        "total_edges": len(lines),
    }


def check_repetition(img_gray: np.ndarray) -> dict:
    """FFT-based repetition / tiling-artifact detection."""
    fft = np.fft.fft2(img_gray.astype(np.float32))
    fft_shifted = np.fft.fftshift(fft)
    magnitude = np.log1p(np.abs(fft_shifted))

    h, w = magnitude.shape
    cy, cx = h // 2, w // 2
    # Zero out DC component.
    magnitude[cy - 5 : cy + 5, cx - 5 : cx + 5] = 0

    mean_mag = np.mean(magnitude)
    std_mag = np.std(magnitude)
    num_peaks = int(np.sum(magnitude > mean_mag + 3 * std_mag))
    peak_ratio = num_peaks / (h * w)

    return {
        "repetition_score": min(peak_ratio * 1000, 1.0),
        "num_peaks": num_peaks,
    }


def check_color(img_rgb: np.ndarray) -> dict:
    """Color distribution analysis (contrast, entropy, value range)."""
    gray = np.mean(img_rgb, axis=2)
    bright = float(np.percentile(gray, 95))
    dark = float(np.percentile(gray, 5))
    contrast_ratio = bright / max(dark, 1.0)

    entropies = []
    for ch in range(3):
        hist, _ = np.histogram(img_rgb[:, :, ch], bins=256, range=(0, 255))
        hist = hist / hist.sum()
        entropy = -np.sum(hist[hist > 0] * np.log2(hist[hist > 0]))
        entropies.append(entropy)

    return {
        "contrast_ratio": float(contrast_ratio),
        "mean_entropy": float(np.mean(entropies)),
        "value_range": float(bright - dark),
    }


def check_straight_lines(img_gray: np.ndarray) -> dict:
    """Count unnaturally long straight lines in terrain."""
    import cv2  # noqa: PLC0415

    edges = cv2.Canny(img_gray, 30, 100)
    lines = cv2.HoughLinesP(
        edges, 1, np.pi / 180, threshold=80, minLineLength=200, maxLineGap=5
    )
    if lines is None:
        return {"long_straight_lines": 0}

    img_width = img_gray.shape[1]
    very_long = 0
    for line in lines:
        x1, y1, x2, y2 = line[0]
        length = np.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2)
        if length > img_width / 4:
            very_long += 1

    return {"long_straight_lines": very_long}


def check_texture_detail(img_gray: np.ndarray, block_size: int = 32) -> dict:
    """Local variance analysis for texture consistency."""
    h, w = img_gray.shape
    variances = []
    for y in range(0, h - block_size, block_size):
        for x in range(0, w - block_size, block_size):
            block = img_gray[y : y + block_size, x : x + block_size].astype(
                np.float32
            )
            variances.append(float(np.var(block)))

    if not variances:
        return {"detail_consistency": 0.0, "flat_patches": 0, "total_patches": 0}

    arr = np.array(variances)
    mean_var = float(np.mean(arr))
    var_of_var = float(np.var(arr))

    return {
        "detail_consistency": float(
            1.0 / (1.0 + var_of_var / max(mean_var**2, 1.0))
        ),
        "flat_patches": int(np.sum(arr < 10)),
        "total_patches": len(variances),
    }


def run_tier1(image_path: str | Path) -> dict:
    """Run all Tier 1 programmatic checks.  Returns combined dict."""
    import cv2  # noqa: PLC0415

    img_bgr = cv2.imread(str(image_path))
    if img_bgr is None:
        raise FileNotFoundError(f"Cannot read image: {image_path}")

    img_gray = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY)
    img_rgb = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)

    results: dict[str, Any] = {}
    results.update(check_seams(img_gray))
    results.update(check_repetition(img_gray))
    results.update(check_color(img_rgb))
    results.update(check_straight_lines(img_gray))
    results.update(check_texture_detail(img_gray))

    results["tier1_pass"] = (
        results["seam_score"] < 0.3
        and results["repetition_score"] < 0.5
        and results["contrast_ratio"] > 1.5
        and results["detail_consistency"] > 0.3
    )
    return results


# ---------------------------------------------------------------------------
# Tier 2: Neural aesthetic scoring (needs GPU + pyiqa)
# ---------------------------------------------------------------------------


def run_tier2(
    image_path: str | Path, reference_dir: str | Path | None = None
) -> dict:
    """Run lightweight neural aesthetic scoring via pyiqa."""
    import pyiqa  # noqa: PLC0415
    import torch  # noqa: PLC0415

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    results: dict[str, Any] = {}

    # NIMA aesthetic score (higher = more aesthetic, typical 3-7).
    try:
        nima = pyiqa.create_metric("nima", device=device)
        results["nima_score"] = float(nima(str(image_path)).item())
    except Exception as exc:
        logger.warning("NIMA scoring failed: %s", exc)
        results["nima_score"] = None

    # BRISQUE technical quality (lower = better quality).
    try:
        brisque = pyiqa.create_metric("brisque", device=device)
        results["brisque_score"] = float(brisque(str(image_path)).item())
    except Exception as exc:
        logger.warning("BRISQUE scoring failed: %s", exc)
        results["brisque_score"] = None

    # CLIP similarity against reference images.
    if reference_dir is not None:
        try:
            results["clip_reference_similarity"] = _clip_reference_score(
                image_path, reference_dir, device
            )
        except Exception as exc:
            logger.warning("CLIP reference scoring failed: %s", exc)

    results["tier2_pass"] = True
    if results.get("nima_score") is not None:
        results["tier2_pass"] = results["nima_score"] > 4.0

    return results


def _clip_reference_score(
    image_path: str | Path,
    reference_dir: str | Path,
    device: Any,
) -> float | None:
    """Average CLIP cosine similarity against all PNGs in *reference_dir*."""
    import clip  # noqa: PLC0415
    import torch  # noqa: PLC0415
    import torch.nn.functional as F  # noqa: PLC0415

    model, preprocess = clip.load("ViT-B/32", device=device)
    ref_dir = Path(reference_dir)

    query = preprocess(Image.open(image_path)).unsqueeze(0).to(device)
    with torch.no_grad():
        query_feat = model.encode_image(query)
        query_feat = F.normalize(query_feat, p=2, dim=-1)

    similarities: list[float] = []
    for ref_path in sorted(ref_dir.glob("*.png")):
        ref = preprocess(Image.open(ref_path)).unsqueeze(0).to(device)
        with torch.no_grad():
            ref_feat = model.encode_image(ref)
            ref_feat = F.normalize(ref_feat, p=2, dim=-1)
        sim = (query_feat @ ref_feat.T).item()
        similarities.append(sim)

    return float(np.mean(similarities)) if similarities else None


# ---------------------------------------------------------------------------
# Tier 3: VLM deep critique (Qwen2.5-VL)
# ---------------------------------------------------------------------------

VISUAL_CRITIC_SYSTEM = (
    "You are a harsh visual quality critic for a dark fantasy isometric action "
    "game (similar to Path of Exile, Diablo). You must be BRUTALLY HONEST about "
    "visual problems. The developer cannot see the game -- you are their eyes."
)

VISUAL_CRITIC_PROMPT = """\
Examine this game screenshot and evaluate EACH of the following. For each item,
answer YES or NO, then explain what you see:

1. TILE SEAMS: Are grid lines or tile boundaries visible?
2. PATTERN REPETITION: Does the same texture repeat obviously?
3. COLOR BANDING: Are there abrupt color transitions?
4. CONTRAST: Is there enough value range?
5. UNNATURAL GEOMETRY: Perfectly straight lines in natural terrain?
6. VISUAL NOISE: Random artifacts, z-fighting, dithering?
7. SCALE CONSISTENCY: Do all elements look the right size?
8. ATMOSPHERE: Does the scene have mood and depth?
9. OVERALL: Compared to commercial dark fantasy ARPGs, score 1-10.

Return JSON:
{
  "tile_seams": {"visible": true, "severity": 1, "details": "..."},
  "pattern_repetition": {"visible": true, "severity": 1, "details": "..."},
  "color_banding": {"visible": false, "severity": 1, "details": "..."},
  "contrast_problems": {"visible": false, "severity": 1, "details": "..."},
  "unnatural_geometry": {"visible": true, "severity": 1, "details": "..."},
  "visual_noise": {"visible": false, "severity": 1, "details": "..."},
  "scale_consistency": {"ok": true, "details": "..."},
  "atmosphere": {"has_mood": false, "has_depth": false, "details": "..."},
  "overall_score": 5,
  "worst_problems": ["most critical issue first"],
  "actionable_fixes": ["specific thing to change first"]
}
"""

VISUAL_COMPARE_PROMPT = """\
You are comparing two versions of the same game scene.
Image 1 is BEFORE a terrain rendering change.
Image 2 is AFTER the change.

For each dimension, state which version is BETTER and why:
1. Tile seam visibility
2. Color naturalness
3. Terrain detail / texture quality
4. Overall visual appeal

Return JSON:
{
  "better_version": "before_or_after",
  "improvements": ["what got better"],
  "regressions": ["what got worse"],
  "recommendation": "keep_or_revert",
  "confidence": 0.8,
  "reasoning": "..."
}
"""


def run_tier3(image_path: str | Path, vision_agent: Any) -> dict:
    """Run VLM deep critique using the existing Qwen2.5-VL agent."""
    try:
        result = vision_agent.analyze_frame(
            image_path,
            VISUAL_CRITIC_SYSTEM,
            VISUAL_CRITIC_PROMPT,
            retries=3,
        )
        result["tier3_pass"] = result.get("overall_score", 0) >= 5
        return result
    except Exception as exc:
        logger.error("VLM critique failed: %s", exc)
        return {"tier3_pass": True, "error": str(exc)}


# ---------------------------------------------------------------------------
# Quality tracking over time
# ---------------------------------------------------------------------------


class VisualQualityTracker:
    """Track visual quality scores across iterations to detect regressions."""

    def __init__(self, log_path: str | Path = "visual_quality.jsonl") -> None:
        self.log_path = Path(log_path)

    def record(
        self,
        screenshot_path: str,
        scores: dict[str, Any],
        commit_hash: str | None = None,
        description: str | None = None,
    ) -> None:
        entry = {
            "timestamp": datetime.now().isoformat(),
            "screenshot": screenshot_path,
            "commit": commit_hash,
            "description": description,
            "scores": scores,
        }
        with self.log_path.open("a") as f:
            f.write(json.dumps(entry) + "\n")

    def get_baseline(self, metric_name: str, n_recent: int = 5) -> float | None:
        """Return average of last *n_recent* values for *metric_name*."""
        if not self.log_path.exists():
            return None

        entries = []
        with self.log_path.open() as f:
            for line in f:
                line = line.strip()
                if line:
                    entries.append(json.loads(line))

        recent = entries[-n_recent:]
        values = [
            e["scores"][metric_name]
            for e in recent
            if metric_name in e.get("scores", {})
            and e["scores"][metric_name] is not None
        ]
        return sum(values) / len(values) if values else None

    def check_regression(
        self, current_scores: dict[str, Any], threshold: float = 0.15
    ) -> list[str]:
        """Return list of regression descriptions (empty = no regression)."""
        lower_is_better = {"brisque_score", "seam_score", "repetition_score"}
        regressions: list[str] = []

        for metric, value in current_scores.items():
            if value is None:
                continue
            baseline = self.get_baseline(metric)
            if baseline is None:
                continue

            if metric in lower_is_better:
                if value > baseline * (1 + threshold):
                    regressions.append(
                        f"{metric}: {baseline:.3f} -> {value:.3f} (worse)"
                    )
            else:
                if value < baseline * (1 - threshold):
                    regressions.append(
                        f"{metric}: {baseline:.3f} -> {value:.3f} (worse)"
                    )

        return regressions


# ---------------------------------------------------------------------------
# Full pipeline
# ---------------------------------------------------------------------------


def evaluate_screenshot(
    image_path: str | Path,
    vision_agent: Any = None,
    reference_dir: str | Path | None = None,
    run_all_tiers: bool = False,
    tracker: VisualQualityTracker | None = None,
) -> QualityReport:
    """Run the full quality assessment pipeline.

    Args:
        image_path: Path to the screenshot to evaluate.
        vision_agent: Optional VisionAgent instance for Tier 3.
        reference_dir: Optional directory of reference screenshots for CLIP.
        run_all_tiers: If True, run all tiers even if earlier ones fail.
        tracker: Optional tracker for regression detection.

    Returns:
        QualityReport with all results.
    """
    report = QualityReport(screenshot=str(image_path))

    # -- Tier 1 ---------------------------------------------------------------
    logger.info("Running Tier 1 (programmatic checks)...")
    t1 = run_tier1(image_path)
    report.seam_score = t1["seam_score"]
    report.repetition_score = t1["repetition_score"]
    report.contrast_ratio = t1["contrast_ratio"]
    report.long_straight_lines = t1["long_straight_lines"]
    report.detail_consistency = t1["detail_consistency"]
    report.tier1_passed = t1["tier1_pass"]

    if not report.tier1_passed and not run_all_tiers:
        report.overall_pass = False
        logger.warning("Tier 1 FAILED -- skipping neural/VLM evaluation.")
        return report

    # -- Tier 2 ---------------------------------------------------------------
    logger.info("Running Tier 2 (neural scoring)...")
    try:
        t2 = run_tier2(image_path, reference_dir)
        report.nima_score = t2.get("nima_score")
        report.brisque_score = t2.get("brisque_score")
        report.clip_reference_similarity = t2.get("clip_reference_similarity")
        report.tier2_passed = t2["tier2_pass"]
    except ImportError:
        logger.warning("pyiqa not installed -- skipping Tier 2. pip install pyiqa")

    if not report.tier2_passed and not run_all_tiers:
        report.overall_pass = False
        logger.warning("Tier 2 FAILED -- skipping VLM evaluation.")
        return report

    # -- Tier 3 ---------------------------------------------------------------
    if vision_agent is not None:
        logger.info("Running Tier 3 (VLM deep critique)...")
        t3 = run_tier3(image_path, vision_agent)
        report.vlm_overall_score = t3.get("overall_score")
        report.vlm_worst_problems = t3.get("worst_problems", [])
        report.vlm_actionable_fixes = t3.get("actionable_fixes", [])
        report.tier3_passed = t3.get("tier3_pass", True)

    # -- Regression check -----------------------------------------------------
    if tracker is not None:
        scores = {
            "seam_score": report.seam_score,
            "repetition_score": report.repetition_score,
            "contrast_ratio": report.contrast_ratio,
            "detail_consistency": report.detail_consistency,
            "nima_score": report.nima_score,
            "brisque_score": report.brisque_score,
        }
        regressions = tracker.check_regression(scores)
        if regressions:
            report.is_regression = True
            report.regression_details = regressions

        tracker.record(str(image_path), scores)

    report.overall_pass = (
        report.tier1_passed and report.tier2_passed and report.tier3_passed
    )
    return report
