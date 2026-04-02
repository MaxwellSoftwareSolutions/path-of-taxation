# Visual Quality Assessment for AI Coding Agents: Research Report

**Date:** 2026-04-01
**Goal:** Build an automated feedback loop so the coding agent detects "this looks bad" before the user has to say it.

---

## 1. VLM-Based Quality Assessment Pipeline (Using Qwen2.5-VL)

### The Core Idea

The existing game agent already loads Qwen2.5-VL and sends screenshots with prompts expecting JSON back. The same infrastructure can run a **visual critic mode** that evaluates aesthetic quality instead of gameplay state.

### Prompts That Get Honest Critique

The key insight from Q-Bench research is that VLMs respond better to **specific, checklist-style questions** than vague "rate the quality" prompts. The model needs to be told what to look for.

**Bad prompt:** "Rate the visual quality of this game screenshot."
**Good prompt:** See the `VISUAL_CRITIC_PROMPT` below.

```
You are a harsh visual quality critic for a dark fantasy isometric action game
(similar to Path of Exile, Diablo). You must be BRUTALLY HONEST about visual
problems. The developer cannot see the game -- you are their eyes.

Examine this screenshot and evaluate EACH of the following. For each item,
answer YES or NO, then explain what you see:

1. TILE SEAMS: Are grid lines or tile boundaries visible? Can you see where
   one tile ends and another begins? Are there hard edges between terrain
   sections?

2. PATTERN REPETITION: Does the same texture/pattern repeat obviously? Can you
   see a "wallpaper" effect where the same small image is tiled visibly?

3. COLOR BANDING: Are there abrupt color transitions instead of smooth
   gradients? Does the terrain look like it has discrete color steps?

4. CONTRAST PROBLEMS: Is the image too flat/washed out, or too dark to see
   detail? Is there enough value range between light and dark areas?

5. UNNATURAL GEOMETRY: Are there perfectly straight lines where there should
   be organic edges? Do shapes look too geometric for natural terrain?

6. VISUAL NOISE: Is there random pixel noise, dithering artifacts, or
   z-fighting (flickering overlapping surfaces)?

7. SCALE CONSISTENCY: Do objects, characters, and terrain elements look like
   they belong at the same scale?

8. ATMOSPHERE: Does the scene have mood? Is there depth (foreground/background
   differentiation)? Or does it look flat and lifeless?

9. OVERALL POLISH: Compared to commercial dark fantasy ARPGs (Path of Exile,
   Diablo, Grim Dawn), how does this look on a 1-10 scale?

Return JSON:
{
  "tile_seams": {"visible": true/false, "severity": 1-5, "details": "..."},
  "pattern_repetition": {"visible": true/false, "severity": 1-5, "details": "..."},
  "color_banding": {"visible": true/false, "severity": 1-5, "details": "..."},
  "contrast_problems": {"visible": true/false, "severity": 1-5, "details": "..."},
  "unnatural_geometry": {"visible": true/false, "severity": 1-5, "details": "..."},
  "visual_noise": {"visible": true/false, "severity": 1-5, "details": "..."},
  "scale_consistency": {"ok": true/false, "details": "..."},
  "atmosphere": {"has_mood": true/false, "has_depth": true/false, "details": "..."},
  "overall_score": 1-10,
  "worst_problems": ["most critical issue first", ...],
  "actionable_fixes": ["specific thing to change first", ...]
}
```

### A/B Comparison Prompt

When comparing two versions (before/after a change):

```
You are comparing two versions of the same game scene. Image 1 is BEFORE,
Image 2 is AFTER a terrain rendering change.

For each dimension, state which version is BETTER and why:
1. Tile seam visibility
2. Color naturalness
3. Terrain detail/texture quality
4. Overall visual appeal

Return JSON:
{
  "better_version": "before" or "after",
  "improvements": ["what got better", ...],
  "regressions": ["what got worse", ...],
  "recommendation": "keep" or "revert",
  "confidence": 0.0-1.0,
  "reasoning": "..."
}
```

---

## 2. Reference Image Comparison

### Available Techniques (ranked by usefulness)

#### CLIP Similarity (BEST for style comparison)
Compares semantic/style similarity. Best for "does my game look like Path of Exile?"

```python
import torch
import clip
from PIL import Image
import torch.nn.functional as F

device = "cuda" if torch.cuda.is_available() else "cpu"
model, preprocess = clip.load("ViT-B/32", device=device)

def clip_similarity(img_path_1, img_path_2):
    img1 = preprocess(Image.open(img_path_1)).unsqueeze(0).to(device)
    img2 = preprocess(Image.open(img_path_2)).unsqueeze(0).to(device)

    with torch.no_grad():
        feat1 = model.encode_image(img1)
        feat2 = model.encode_image(img2)

    feat1 = F.normalize(feat1, p=2, dim=-1)
    feat2 = F.normalize(feat2, p=2, dim=-1)

    return (feat1 @ feat2.T).item()  # -1 to 1, higher = more similar
```

**Practical use:** Collect 20-50 screenshots from PoE2/Diablo/Grim Dawn terrain.
Compute average CLIP similarity between your game and each reference set.
Track this score over time -- it should go UP as terrain improves.

#### LPIPS (BEST for detecting perceptual differences between versions)
Measures perceptual distance. Best for "did my change make things better or worse?"

```python
import lpips
import torch
from PIL import Image
import torchvision.transforms as T

loss_fn = lpips.LPIPS(net='alex')  # 'alex' is fastest

transform = T.Compose([
    T.Resize((256, 256)),
    T.ToTensor(),
    T.Normalize(mean=[0.5]*3, std=[0.5]*3),  # normalize to [-1, 1]
])

def perceptual_distance(img_path_1, img_path_2):
    img1 = transform(Image.open(img_path_1)).unsqueeze(0)
    img2 = transform(Image.open(img_path_2)).unsqueeze(0)
    with torch.no_grad():
        return loss_fn(img1, img2).item()  # 0 = identical, higher = more different
```

#### SSIM (BEST for detecting structural artifacts like seams)
Structural similarity. Good for checking if terrain has unwanted structural patterns.

```python
from skimage.metrics import structural_similarity as ssim
from skimage import io, img_as_float

def compute_ssim(img_path_1, img_path_2):
    img1 = img_as_float(io.imread(img_path_1))
    img2 = img_as_float(io.imread(img_path_2))
    return ssim(img1, img2, data_range=1.0, channel_axis=2)
    # 1.0 = identical, lower = more different
```

**Creative use for seam detection:** Compare a terrain screenshot against a copy of
itself shifted by one tile width. If SSIM is very high, the terrain is too repetitive.

#### FID (LEAST useful for single-image comparison)
FID compares distributions of images, not individual images. Only useful if you have
50+ screenshots of your game AND 50+ screenshots of reference games. Skip this for
now unless building a large-scale evaluation.

### Recommended Reference Image Setup

Create `/home/hex/path-of-taxation/game_agent/reference_images/` with subdirs:
- `poe2_terrain/` -- 20+ PoE2 screenshots of terrain close-ups
- `diablo_terrain/` -- 20+ Diablo terrain screenshots
- `grim_dawn_terrain/` -- 20+ Grim Dawn terrain
- `our_best/` -- Screenshots of your game when it looked good (curated by user)
- `our_worst/` -- Screenshots when it looked bad (for negative reference)

---

## 3. Game-Specific Visual Quality Heuristics (Programmatic)

These are concrete, code-measurable metrics that do NOT require a neural network.

### 3.1 Tile Seam Detection via Edge Analysis

Tile seams appear as straight horizontal/vertical lines at regular intervals.
Detect them with Canny edge detection + Hough line transform:

```python
import cv2
import numpy as np

def detect_tile_seams(image_path, expected_tile_size=64):
    """Detect regular grid-aligned edges that suggest visible tile seams."""
    img = cv2.imread(image_path, cv2.IMREAD_GRAYSCALE)
    edges = cv2.Canny(img, 50, 150)

    # Hough lines to find straight lines
    lines = cv2.HoughLinesP(edges, 1, np.pi/180, threshold=100,
                            minLineLength=100, maxLineGap=10)
    if lines is None:
        return {"seam_score": 0, "h_lines": 0, "v_lines": 0}

    h_lines = 0
    v_lines = 0
    for line in lines:
        x1, y1, x2, y2 = line[0]
        angle = abs(np.arctan2(y2-y1, x2-x1) * 180 / np.pi)
        if angle < 5 or angle > 175:  # near-horizontal
            h_lines += 1
        elif 85 < angle < 95:  # near-vertical
            v_lines += 1

    # Check if lines fall on tile boundaries
    grid_aligned = 0
    for line in lines:
        x1, y1, x2, y2 = line[0]
        if y1 % expected_tile_size < 3 or x1 % expected_tile_size < 3:
            grid_aligned += 1

    total = len(lines)
    seam_score = grid_aligned / max(total, 1)  # 0-1, higher = more seams

    return {
        "seam_score": seam_score,
        "h_lines": h_lines,
        "v_lines": v_lines,
        "grid_aligned_lines": grid_aligned,
        "total_edges": total
    }
```

### 3.2 Pattern Repetition Detection via FFT

Repeating tile patterns create distinct peaks in the frequency domain:

```python
import numpy as np
from PIL import Image

def detect_repetition(image_path):
    """Use FFT to detect periodic repetition patterns (tiling artifacts)."""
    img = np.array(Image.open(image_path).convert('L'), dtype=np.float32)

    # 2D FFT
    fft = np.fft.fft2(img)
    fft_shifted = np.fft.fftshift(fft)
    magnitude = np.log1p(np.abs(fft_shifted))

    # Remove DC component (center)
    h, w = magnitude.shape
    cy, cx = h // 2, w // 2
    magnitude[cy-5:cy+5, cx-5:cx+5] = 0

    # Strong periodic signals appear as bright peaks
    # A natural image has a smooth falloff; tiled images have spikes
    mean_mag = np.mean(magnitude)
    std_mag = np.std(magnitude)
    peak_threshold = mean_mag + 3 * std_mag
    num_peaks = np.sum(magnitude > peak_threshold)

    # Peak ratio: natural images have few peaks relative to total pixels
    peak_ratio = num_peaks / (h * w)

    # Detect dominant frequency (tells us the tile size)
    peak_coords = np.argwhere(magnitude > peak_threshold)
    if len(peak_coords) > 0:
        distances = np.sqrt((peak_coords[:, 0] - cy)**2 + (peak_coords[:, 1] - cx)**2)
        dominant_freq = np.median(distances)
        estimated_tile_size = h / dominant_freq if dominant_freq > 0 else 0
    else:
        estimated_tile_size = 0

    return {
        "repetition_score": min(peak_ratio * 1000, 1.0),  # 0-1, higher = more repetitive
        "num_frequency_peaks": int(num_peaks),
        "estimated_tile_size": float(estimated_tile_size),
        "interpretation": "high" if peak_ratio > 0.001 else "low"
    }
```

### 3.3 Color Distribution Analysis

Good terrain has smooth color variation. Bad terrain has clumped histograms:

```python
import numpy as np
from PIL import Image

def analyze_color_distribution(image_path):
    """Analyze whether color distribution looks natural or artificial."""
    img = np.array(Image.open(image_path).convert('RGB'))

    results = {}

    # Per-channel histogram analysis
    for i, channel in enumerate(['red', 'green', 'blue']):
        hist, _ = np.histogram(img[:, :, i], bins=256, range=(0, 255))
        hist = hist / hist.sum()  # normalize

        # Entropy: higher = more diverse colors = generally better
        entropy = -np.sum(hist[hist > 0] * np.log2(hist[hist > 0]))

        # Number of distinct values used (out of 256)
        unique_values = np.sum(hist > 0)

        results[f'{channel}_entropy'] = float(entropy)
        results[f'{channel}_unique_values'] = int(unique_values)

    # Overall value range (brightness)
    gray = np.mean(img, axis=2)
    value_range = float(np.percentile(gray, 95) - np.percentile(gray, 5))

    # Contrast ratio (between brightest and darkest significant areas)
    bright = np.percentile(gray, 95)
    dark = np.percentile(gray, 5)
    contrast_ratio = float(bright / max(dark, 1))

    # Hue analysis (in HSV space)
    from colorsys import rgb_to_hsv
    hsv_img = np.array(Image.open(image_path).convert('HSV'))
    hue_std = float(np.std(hsv_img[:, :, 0]))
    sat_mean = float(np.mean(hsv_img[:, :, 1]))

    results.update({
        "value_range": value_range,
        "contrast_ratio": contrast_ratio,
        "hue_diversity": hue_std,
        "saturation_mean": sat_mean,
        # Heuristic thresholds for "looks good"
        "contrast_ok": contrast_ratio > 2.0 and contrast_ratio < 20.0,
        "diversity_ok": hue_std > 15.0,  # Too low = monotone
    })

    return results
```

### 3.4 Straight-Line Detection in Natural Terrain

Natural terrain should have irregular edges. Lots of straight lines = bad:

```python
import cv2
import numpy as np

def detect_unnatural_lines(image_path):
    """Count long straight lines that shouldn't exist in natural terrain."""
    img = cv2.imread(image_path, cv2.IMREAD_GRAYSCALE)
    edges = cv2.Canny(img, 30, 100)

    # Detect long straight lines (>200px)
    lines = cv2.HoughLinesP(edges, 1, np.pi/180, threshold=80,
                            minLineLength=200, maxLineGap=5)
    if lines is None:
        return {"long_straight_lines": 0, "unnaturalness_score": 0.0}

    long_lines = len(lines)

    # Lines longer than 1/4 of image width are very suspicious in terrain
    img_width = img.shape[1]
    very_long = 0
    for line in lines:
        x1, y1, x2, y2 = line[0]
        length = np.sqrt((x2-x1)**2 + (y2-y1)**2)
        if length > img_width / 4:
            very_long += 1

    return {
        "long_straight_lines": long_lines,
        "very_long_lines": very_long,
        "unnaturalness_score": min(very_long / 10.0, 1.0)
    }
```

### 3.5 Local Variance Analysis (Texture Detail)

Good terrain has consistent local detail. Flat patches next to detailed patches = bad:

```python
import numpy as np
from PIL import Image

def analyze_texture_detail(image_path, block_size=32):
    """Measure variance of local texture detail across the image."""
    img = np.array(Image.open(image_path).convert('L'), dtype=np.float32)
    h, w = img.shape

    variances = []
    for y in range(0, h - block_size, block_size):
        for x in range(0, w - block_size, block_size):
            block = img[y:y+block_size, x:x+block_size]
            variances.append(np.var(block))

    variances = np.array(variances)

    return {
        "mean_local_variance": float(np.mean(variances)),
        "variance_of_variance": float(np.var(variances)),
        # High variance-of-variance = inconsistent detail = usually bad
        "detail_consistency": float(1.0 / (1.0 + np.var(variances) / max(np.mean(variances)**2, 1))),
        "flat_patches": int(np.sum(variances < 10)),
        "detailed_patches": int(np.sum(variances > 500)),
        "total_patches": len(variances),
    }
```

---

## 4. Pre-Trained Aesthetic Scoring Models

### 4.1 PyIQA (RECOMMENDED -- single pip install, many metrics)

The `pyiqa` library wraps dozens of IQA models behind a unified API:

```bash
pip install pyiqa
```

```python
import pyiqa
import torch

device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

# No-reference quality metrics (no comparison image needed)
brisque = pyiqa.create_metric('brisque', device=device)
nima = pyiqa.create_metric('nima', device=device)
musiq = pyiqa.create_metric('musiq', device=device)
clipiqa = pyiqa.create_metric('clipiqa', device=device)

# Score a single image (use file path directly)
score = nima('./screenshot.png')
print(f"NIMA aesthetic score: {score.item():.3f}")
print(f"Lower is better: {nima.lower_better}")
```

**Best metrics for game screenshots:**

| Metric    | What it measures              | Lower=better? | Notes                          |
|-----------|-------------------------------|----------------|--------------------------------|
| `nima`    | Aesthetic appeal              | No             | Trained on AVA dataset         |
| `musiq`   | Multi-scale quality           | No             | Handles arbitrary resolution   |
| `clipiqa` | CLIP-based quality            | No             | Good for style assessment      |
| `brisque` | Technical quality (artifacts) | Yes            | Good for detecting distortion  |
| `topiq_nr`| Modern NR quality             | No             | State-of-the-art NR metric     |

### 4.2 LAION Aesthetic Predictor (CLIP-based, lightweight)

Scores images on a ~1-10 scale using a small MLP on top of CLIP embeddings:

```python
import torch
import clip
from PIL import Image
import torch.nn as nn

class AestheticMLP(nn.Module):
    def __init__(self, input_size=768):
        super().__init__()
        self.layers = nn.Sequential(
            nn.Linear(input_size, 1024),
            nn.Dropout(0.2),
            nn.Linear(1024, 128),
            nn.Dropout(0.2),
            nn.Linear(128, 64),
            nn.Dropout(0.1),
            nn.Linear(64, 16),
            nn.Linear(16, 1),
        )
    def forward(self, x):
        return self.layers(x)

# Load models
device = "cuda"
clip_model, preprocess = clip.load("ViT-L/14", device=device)
aesthetic_model = AestheticMLP(768)
# Download weights from: https://github.com/christophschuhmann/improved-aesthetic-predictor
aesthetic_model.load_state_dict(torch.load("sac+logos+ava1-l14-linearMSE.pth"))
aesthetic_model.to(device).eval()

def aesthetic_score(image_path):
    image = preprocess(Image.open(image_path)).unsqueeze(0).to(device)
    with torch.no_grad():
        features = clip_model.encode_image(image)
        features = features / features.norm(dim=-1, keepdim=True)  # L2 normalize
        score = aesthetic_model(features.float())
    return score.item()  # ~1-10, higher = more aesthetic
```

### 4.3 Q-Align / OneAlign (quality + aesthetics from one model)

Heavyweight but very capable. Outputs 1-5 scale for both quality AND aesthetics:

```python
import torch
from transformers import AutoModelForCausalLM
from PIL import Image

model = AutoModelForCausalLM.from_pretrained(
    "q-future/one-align",
    trust_remote_code=True,
    torch_dtype=torch.float16,
    device_map="auto"
)

image = Image.open("screenshot.png")

# Quality score (1-5, higher = better technical quality)
quality = model.score([image], task_="quality", input_="image")

# Aesthetic score (1-5, higher = more aesthetically pleasing)
aesthetics = model.score([image], task_="aesthetics", input_="image")
```

**Warning:** Q-Align is large (~7B params). If Qwen2.5-VL is already loaded, there
may not be enough VRAM for both. Consider running them in separate passes.

---

## 5. Iterative Visual Refinement Workflow

### The Complete Pipeline

```
[Code Change] --> [Build Game] --> [Launch Game] --> [Screenshot]
                                                         |
                                                         v
                                              +-------------------+
                                              | FAST CHECKS       |
                                              | (no neural net)   |
                                              |                   |
                                              | - Seam detection  |
                                              | - FFT repetition  |
                                              | - Color analysis  |
                                              | - Line detection  |
                                              +--------+----------+
                                                       |
                                              PASS?    |    FAIL? --> immediate feedback
                                                       v
                                              +-------------------+
                                              | AESTHETIC SCORES  |
                                              | (lightweight NN)  |
                                              |                   |
                                              | - NIMA score      |
                                              | - BRISQUE score   |
                                              | - CLIP similarity |
                                              |   vs reference    |
                                              +--------+----------+
                                                       |
                                              ABOVE    |    BELOW
                                              THRESHOLD?    THRESHOLD --> feedback
                                                       v
                                              +-------------------+
                                              | VLM DEEP CRITIQUE |
                                              | (Qwen2.5-VL)     |
                                              |                   |
                                              | - Checklist eval  |
                                              | - A/B comparison  |
                                              | - Fix suggestions |
                                              +--------+----------+
                                                       |
                                                       v
                                              [Report to Agent]
```

### Implementation: Three-Tier Evaluation

**Tier 1: Programmatic checks (< 1 second, no GPU)**
- Seam detection, FFT analysis, color stats, line detection
- These catch the most obvious problems instantly
- If ANY score is in the "critical" range, stop and report immediately

**Tier 2: Lightweight neural scoring (< 5 seconds, GPU)**
- PyIQA metrics: NIMA, BRISQUE, CLIP-IQA
- CLIP similarity against reference images
- Compare against stored baseline scores from previous "good" versions

**Tier 3: VLM deep analysis (10-30 seconds, GPU)**
- Only run when Tier 1+2 pass OR for final validation
- Full checklist critique with Qwen2.5-VL
- A/B comparison with previous version
- Generate specific, actionable fix suggestions

### Score Tracking Over Time

```python
import json
from datetime import datetime
from pathlib import Path

class VisualQualityTracker:
    """Track visual quality scores across iterations."""

    def __init__(self, log_path="visual_quality_log.jsonl"):
        self.log_path = Path(log_path)

    def record(self, screenshot_path, scores, commit_hash=None, description=None):
        entry = {
            "timestamp": datetime.now().isoformat(),
            "screenshot": str(screenshot_path),
            "commit": commit_hash,
            "description": description,
            "scores": scores,
        }
        with self.log_path.open("a") as f:
            f.write(json.dumps(entry) + "\n")

    def get_baseline(self, metric_name, n_recent=5):
        """Get average of last N scores for a metric."""
        entries = []
        if self.log_path.exists():
            with self.log_path.open() as f:
                for line in f:
                    entries.append(json.loads(line))
        recent = entries[-n_recent:]
        values = [e["scores"].get(metric_name) for e in recent if metric_name in e.get("scores", {})]
        return sum(values) / len(values) if values else None

    def is_regression(self, current_scores, threshold=0.15):
        """Check if current scores represent a visual regression."""
        regressions = []
        for metric, value in current_scores.items():
            baseline = self.get_baseline(metric)
            if baseline is None:
                continue
            # For "lower is better" metrics
            if metric in ("brisque", "seam_score", "repetition_score"):
                if value > baseline * (1 + threshold):
                    regressions.append(f"{metric}: {baseline:.3f} -> {value:.3f} (WORSE)")
            # For "higher is better" metrics
            else:
                if value < baseline * (1 - threshold):
                    regressions.append(f"{metric}: {baseline:.3f} -> {value:.3f} (WORSE)")
        return regressions
```

---

## 6. Art Direction Vocabulary for Dark Fantasy Terrain

### Visual Quality Vocabulary (for prompts and self-evaluation)

**Positive descriptors (what you want):**
- **Value depth** -- wide range from deep blacks to bright highlights
- **Atmospheric perspective** -- distant terrain is hazier/less saturated
- **Organic edges** -- irregular, weathered boundaries between terrain types
- **Material definition** -- stone looks like stone, dirt looks like dirt
- **Micro-detail** -- small cracks, pebbles, moss patches that break up flat areas
- **Color temperature variation** -- warm/cool shifts across the terrain
- **Ambient occlusion** -- darker where surfaces meet (crevices, corners)
- **Ground plane readability** -- you can tell what's walkable vs. obstacle
- **Depth layering** -- foreground, midground, background distinguishable
- **Grime and weathering** -- nothing looks "new" in a dark fantasy world

**Negative descriptors (what you want to detect and fix):**
- **Tile tiling** (visible grid) -- obvious repeating pattern
- **Seam bleeding** -- color mismatch at tile boundaries
- **Flat lighting** -- no shadows, no highlights, everything same brightness
- **Chromatic monotony** -- everything is the same hue
- **Checkerboard artifact** -- alternating light/dark blocks
- **UV swimming** -- textures that shift when camera moves
- **Z-fighting** -- flickering where two surfaces overlap
- **Moire patterns** -- interference patterns from overlapping fine detail
- **Pillow shading** -- tiles that are light in center, dark at edges
- **Wallpaper effect** -- obvious repeating pattern across large area

### Color Palette Rules for Dark Fantasy

- **Primary range:** Desaturated earth tones (browns, grays, muted greens)
- **Value range:** 15-85% brightness (never pure black floor, never pure white)
- **Accent colors:** Deep reds, sickly greens, cold blues (used sparingly, < 10%)
- **Saturation:** Generally low (20-40%), with occasional pops (60-80%) for magic/blood/important elements
- **Temperature:** Cool shadows, warm midtones -- or the reverse for otherworldly areas
- **Hue variation:** Even "gray stone" should have subtle blue/purple/warm shifts per tile

### Dark Fantasy Terrain Checklist (for VLM prompt or manual review)

1. Can I see where tiles repeat? (NO = good)
2. Are there at least 3 distinct value levels visible? (YES = good)
3. Do terrain edges look organic/irregular? (YES = good)
4. Is there color temperature variation? (YES = good)
5. Can I distinguish terrain types (stone vs dirt vs grass)? (YES = good)
6. Do shadows exist and make spatial sense? (YES = good)
7. Is there any visual noise/artifacting? (NO = good)
8. Does the overall mood feel dark/atmospheric? (YES = good)
9. Could this pass as a screenshot from Grim Dawn? (YES = goal)

---

## 7. Practical Integration Plan

### What Needs to Change in the Existing Game Agent

The game agent at `/home/hex/path-of-taxation/game_agent/` needs a new mode and new modules.

### New Files to Create

#### `visual_critic.py` -- The main critic module

```python
"""Visual quality critic for terrain and game scene evaluation.

Runs a three-tier quality assessment pipeline:
  Tier 1: Fast programmatic checks (seams, repetition, color, lines)
  Tier 2: Neural aesthetic scoring (NIMA, BRISQUE via pyiqa)
  Tier 3: VLM deep critique (Qwen2.5-VL checklist evaluation)
"""

from __future__ import annotations

import json
import logging
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


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
            lines.append(f"  Tier 2 (neural): {'PASS' if self.tier2_passed else 'FAIL'}")
            lines.append(f"    NIMA aesthetic: {self.nima_score:.3f}")
            lines.append(f"    BRISQUE quality: {self.brisque_score:.3f}")
        if self.vlm_overall_score is not None:
            lines.append(f"  Tier 3 (VLM): {'PASS' if self.tier3_passed else 'FAIL'}")
            lines.append(f"    VLM score: {self.vlm_overall_score}/10")
            if self.vlm_worst_problems:
                lines.append(f"    Worst problems: {', '.join(self.vlm_worst_problems[:3])}")
            if self.vlm_actionable_fixes:
                lines.append(f"    Suggested fixes: {', '.join(self.vlm_actionable_fixes[:3])}")
        if self.is_regression:
            lines.append(f"  REGRESSION DETECTED: {'; '.join(self.regression_details)}")
        return "\n".join(lines)


# ---- Tier 1: Programmatic Checks ----

def check_seams(img_gray: np.ndarray, expected_tile_size: int = 64) -> dict:
    """Detect tile seams via Canny + Hough line transform."""
    import cv2
    edges = cv2.Canny(img_gray, 50, 150)
    lines = cv2.HoughLinesP(edges, 1, np.pi/180, threshold=100,
                            minLineLength=100, maxLineGap=10)
    if lines is None:
        return {"seam_score": 0.0, "grid_aligned": 0}

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
    """FFT-based repetition detection."""
    fft = np.fft.fft2(img_gray.astype(np.float32))
    fft_shifted = np.fft.fftshift(fft)
    magnitude = np.log1p(np.abs(fft_shifted))

    h, w = magnitude.shape
    cy, cx = h // 2, w // 2
    magnitude[cy-5:cy+5, cx-5:cx+5] = 0

    mean_mag = np.mean(magnitude)
    std_mag = np.std(magnitude)
    num_peaks = int(np.sum(magnitude > mean_mag + 3 * std_mag))
    peak_ratio = num_peaks / (h * w)

    return {
        "repetition_score": min(peak_ratio * 1000, 1.0),
        "num_peaks": num_peaks,
    }


def check_color(img_rgb: np.ndarray) -> dict:
    """Color distribution analysis."""
    gray = np.mean(img_rgb, axis=2)
    bright = float(np.percentile(gray, 95))
    dark = float(np.percentile(gray, 5))
    contrast_ratio = bright / max(dark, 1)

    # Per-channel entropy
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
    """Count unnaturally long straight lines."""
    import cv2
    edges = cv2.Canny(img_gray, 30, 100)
    lines = cv2.HoughLinesP(edges, 1, np.pi/180, threshold=80,
                            minLineLength=200, maxLineGap=5)
    if lines is None:
        return {"long_straight_lines": 0}

    img_width = img_gray.shape[1]
    very_long = 0
    for line in lines:
        x1, y1, x2, y2 = line[0]
        length = np.sqrt((x2-x1)**2 + (y2-y1)**2)
        if length > img_width / 4:
            very_long += 1

    return {"long_straight_lines": very_long}


def check_texture_detail(img_gray: np.ndarray, block_size: int = 32) -> dict:
    """Local variance analysis for texture consistency."""
    h, w = img_gray.shape
    variances = []
    for y in range(0, h - block_size, block_size):
        for x in range(0, w - block_size, block_size):
            block = img_gray[y:y+block_size, x:x+block_size].astype(np.float32)
            variances.append(float(np.var(block)))

    variances = np.array(variances)
    mean_var = float(np.mean(variances)) if len(variances) > 0 else 0
    var_of_var = float(np.var(variances)) if len(variances) > 0 else 0

    return {
        "detail_consistency": float(1.0 / (1.0 + var_of_var / max(mean_var**2, 1))),
        "flat_patches": int(np.sum(variances < 10)),
        "total_patches": len(variances),
    }


def run_tier1(image_path: str | Path) -> dict:
    """Run all programmatic checks. Returns combined results."""
    import cv2
    img_bgr = cv2.imread(str(image_path))
    img_gray = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY)
    img_rgb = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2RGB)

    results = {}
    results.update(check_seams(img_gray))
    results.update(check_repetition(img_gray))
    results.update(check_color(img_rgb))
    results.update(check_straight_lines(img_gray))
    results.update(check_texture_detail(img_gray))

    # Determine pass/fail
    results["tier1_pass"] = (
        results["seam_score"] < 0.3
        and results["repetition_score"] < 0.5
        and results["contrast_ratio"] > 1.5
        and results["detail_consistency"] > 0.3
    )
    return results


# ---- Tier 2: Neural Scoring ----

def run_tier2(image_path: str | Path, reference_dir: str | Path | None = None) -> dict:
    """Run lightweight neural aesthetic scoring."""
    import pyiqa
    import torch

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    results = {}

    # NIMA aesthetic score
    try:
        nima = pyiqa.create_metric('nima', device=device)
        results["nima_score"] = float(nima(str(image_path)).item())
        results["nima_lower_better"] = bool(nima.lower_better)
    except Exception as e:
        logger.warning("NIMA scoring failed: %s", e)
        results["nima_score"] = None

    # BRISQUE technical quality
    try:
        brisque = pyiqa.create_metric('brisque', device=device)
        results["brisque_score"] = float(brisque(str(image_path)).item())
    except Exception as e:
        logger.warning("BRISQUE scoring failed: %s", e)
        results["brisque_score"] = None

    # CLIP similarity against reference images
    if reference_dir is not None:
        try:
            results["clip_reference_similarity"] = _clip_reference_score(
                image_path, reference_dir, device
            )
        except Exception as e:
            logger.warning("CLIP reference scoring failed: %s", e)

    results["tier2_pass"] = True
    if results.get("nima_score") is not None:
        # NIMA: higher is better, typical range 3-7
        results["tier2_pass"] = results["nima_score"] > 4.0

    return results


def _clip_reference_score(image_path, reference_dir, device):
    """Compute average CLIP similarity against reference images."""
    import clip
    import torch.nn.functional as F

    model, preprocess = clip.load("ViT-B/32", device=device)
    ref_dir = Path(reference_dir)

    query = preprocess(Image.open(image_path)).unsqueeze(0).to(device)
    with torch.no_grad():
        query_feat = model.encode_image(query)
        query_feat = F.normalize(query_feat, p=2, dim=-1)

    similarities = []
    for ref_path in ref_dir.glob("*.png"):
        ref = preprocess(Image.open(ref_path)).unsqueeze(0).to(device)
        with torch.no_grad():
            ref_feat = model.encode_image(ref)
            ref_feat = F.normalize(ref_feat, p=2, dim=-1)
        sim = (query_feat @ ref_feat.T).item()
        similarities.append(sim)

    return float(np.mean(similarities)) if similarities else None


# ---- Tier 3: VLM Deep Critique ----

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
  "tile_seams": {"visible": true/false, "severity": 1-5, "details": "..."},
  "pattern_repetition": {"visible": true/false, "severity": 1-5, "details": "..."},
  "color_banding": {"visible": true/false, "severity": 1-5, "details": "..."},
  "contrast_problems": {"visible": true/false, "severity": 1-5, "details": "..."},
  "unnatural_geometry": {"visible": true/false, "severity": 1-5, "details": "..."},
  "visual_noise": {"visible": true/false, "severity": 1-5, "details": "..."},
  "scale_consistency": {"ok": true/false, "details": "..."},
  "atmosphere": {"has_mood": true/false, "has_depth": true/false, "details": "..."},
  "overall_score": 1-10,
  "worst_problems": ["most critical issue first", "..."],
  "actionable_fixes": ["specific thing to change first", "..."]
}
"""

def run_tier3(image_path: str | Path, vision_agent) -> dict:
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
    except Exception as e:
        logger.error("VLM critique failed: %s", e)
        return {"tier3_pass": True, "error": str(e)}


# ---- Full Pipeline ----

def evaluate_screenshot(
    image_path: str | Path,
    vision_agent=None,
    reference_dir: str | Path | None = None,
    run_all_tiers: bool = False,
) -> QualityReport:
    """Run the full quality assessment pipeline.

    Args:
        image_path: Path to the screenshot to evaluate.
        vision_agent: Optional VisionAgent instance for Tier 3.
        reference_dir: Optional directory of reference screenshots for CLIP comparison.
        run_all_tiers: If True, run all tiers regardless of pass/fail.
                       If False, stop at first failing tier.

    Returns:
        QualityReport with all results.
    """
    report = QualityReport(screenshot=str(image_path))

    # Tier 1: Programmatic
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

    # Tier 2: Neural scoring
    logger.info("Running Tier 2 (neural scoring)...")
    try:
        t2 = run_tier2(image_path, reference_dir)
        report.nima_score = t2.get("nima_score")
        report.brisque_score = t2.get("brisque_score")
        report.clip_reference_similarity = t2.get("clip_reference_similarity")
        report.tier2_passed = t2["tier2_pass"]
    except ImportError:
        logger.warning("pyiqa not installed -- skipping Tier 2.")

    if not report.tier2_passed and not run_all_tiers:
        report.overall_pass = False
        logger.warning("Tier 2 FAILED -- skipping VLM evaluation.")
        return report

    # Tier 3: VLM critique
    if vision_agent is not None:
        logger.info("Running Tier 3 (VLM deep critique)...")
        t3 = run_tier3(image_path, vision_agent)
        report.vlm_overall_score = t3.get("overall_score")
        report.vlm_worst_problems = t3.get("worst_problems", [])
        report.vlm_actionable_fixes = t3.get("actionable_fixes", [])
        report.tier3_passed = t3.get("tier3_pass", True)

    report.overall_pass = report.tier1_passed and report.tier2_passed and report.tier3_passed
    return report
```

#### Changes to `prompts.py` -- Add the critic prompt

Add the `VISUAL_CRITIC_SYSTEM` and `VISUAL_CRITIC_PROMPT` as importable constants
(already defined in `visual_critic.py` above).

#### Changes to `main.py` -- Add `--critic` mode

Add a `--critic` CLI flag that evaluates a single screenshot or runs the critic
after each game loop tick:

```
python -m game_agent --mode observe --critic
python -m game_agent --critic-only /path/to/screenshot.png
```

#### Changes to `config.py` -- Add critic configuration

```python
# Visual critic settings
REFERENCE_IMAGE_DIR = BASE_DIR / "reference_images"
VISUAL_QUALITY_LOG = LOG_DIR / "visual_quality.jsonl"
CRITIC_THRESHOLDS = {
    "seam_score_max": 0.3,
    "repetition_score_max": 0.5,
    "contrast_ratio_min": 1.5,
    "nima_score_min": 4.0,
    "vlm_overall_min": 5,
}
```

#### New dependencies to add to `requirements.txt`

```
pyiqa>=0.1.10
opencv-python-headless>=4.8.0
scikit-image>=0.21.0
lpips>=0.1.4
open-clip-torch>=2.20.0
```

### Integration with the Coding Agent Workflow

The key integration point is NOT in the game agent itself, but in the **coding
agent's workflow** (Claude). After making terrain changes:

1. Build the game: `cargo build`
2. Launch briefly: run the game for 3 seconds to get a screenshot
3. Take screenshot via the game agent's `capture_window()`
4. Run `evaluate_screenshot()` on the captured image
5. Read the `QualityReport.summary()`
6. If FAIL: adjust the code and repeat
7. If PASS: proceed to commit

This can be automated as a hook that runs before any terrain-related commit.

### CLI Tool for Quick Evaluation

Create a standalone script at `/home/hex/path-of-taxation/game_agent/critic_cli.py`:

```python
#!/usr/bin/env python3
"""Quick visual quality check on a screenshot.

Usage:
    python critic_cli.py screenshot.png
    python critic_cli.py screenshot.png --tier 1  # fast checks only
    python critic_cli.py screenshot.png --tier 2  # fast + neural
    python critic_cli.py screenshot.png --tier 3  # all tiers
    python critic_cli.py screenshot.png --reference ./reference_images/
    python critic_cli.py --compare before.png after.png
"""
import argparse
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Visual quality critic")
    parser.add_argument("screenshot", type=Path, help="Screenshot to evaluate")
    parser.add_argument("--tier", type=int, default=2, choices=[1, 2, 3])
    parser.add_argument("--reference", type=Path, default=None)
    parser.add_argument("--json", action="store_true", help="Output raw JSON")
    args = parser.parse_args()

    from visual_critic import evaluate_screenshot, run_tier1

    if args.tier == 1:
        result = run_tier1(args.screenshot)
        if args.json:
            import json
            print(json.dumps(result, indent=2))
        else:
            for k, v in result.items():
                print(f"  {k}: {v}")
        sys.exit(0 if result["tier1_pass"] else 1)

    # Tier 2+
    vision_agent = None
    if args.tier >= 3:
        from vision_agent import VisionAgent
        from config import MODEL_ID, MAX_NEW_TOKENS, TEMPERATURE
        vision_agent = VisionAgent(MODEL_ID, max_new_tokens=MAX_NEW_TOKENS,
                                   temperature=TEMPERATURE)

    report = evaluate_screenshot(
        args.screenshot,
        vision_agent=vision_agent,
        reference_dir=args.reference,
        run_all_tiers=True,
    )

    if args.json:
        import json
        print(json.dumps(report.to_dict(), indent=2))
    else:
        print(report.summary())

    sys.exit(0 if report.overall_pass else 1)


if __name__ == "__main__":
    main()
```

---

## Summary of Recommendations

### Immediate (do first, highest impact):
1. **Add `visual_critic.py`** with the Tier 1 programmatic checks -- zero new dependencies (just numpy + opencv), runs in < 1 second, catches the most obvious problems (seams, repetition, flat contrast).
2. **Add the VLM critic prompt** to the existing Qwen2.5-VL setup -- this is the most impactful single change. The checklist prompt above forces the model to evaluate specific problems instead of giving vague "looks fine" answers.
3. **Collect 20+ reference screenshots** from PoE2/Diablo/Grim Dawn terrain close-ups into `reference_images/`.

### Short-term (next session):
4. **Install `pyiqa`** and add NIMA + BRISQUE scoring (Tier 2).
5. **Build the `critic_cli.py`** standalone tool.
6. **Add score tracking** via `VisualQualityTracker` to detect regressions.

### Medium-term (when terrain work is active):
7. **Integrate into the build workflow** -- after every `cargo build`, auto-screenshot and evaluate.
8. **Set up CLIP similarity** against reference image sets.
9. **A/B comparison mode** -- store "before" screenshot, make change, compare "after".

### Things NOT worth doing:
- **FID:** Requires 50+ images per distribution. Overkill for single-screenshot evaluation.
- **Q-Align/OneAlign:** Too heavy to run alongside Qwen2.5-VL. The VLM critic prompt with Qwen achieves the same goal.
- **LAION Aesthetic Predictor:** Trained on photos, not game art. NIMA via pyiqa is better calibrated.
- **MUSIQ:** TensorFlow-based, awkward to integrate into a PyTorch pipeline. Use pyiqa's `musiq` wrapper if needed.
