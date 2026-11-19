"""Animation frames dumped by the ignored tests in src/tui/demo.rs."""
import os
import subprocess
import tempfile

from _common import REPO

# src/tui/anim.rs runs at 7 fps.
FRAME_MS = 1000 // 7
# The cat's paw and pendulum repeat every 8 frames and the clock's colon
# blinks every 7, so this many frames loop without a seam.
FULL_CYCLE = 56
# Starting two frames in makes each loop end with the paw raised, which is
# also the frame left showing once the animation stops.
START = 2


def load(test, count, **env):
    """`count` frames from `test`, each a list of ANSI lines."""
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "frames.ansi")
        env = dict(os.environ, CLOCK_IN_FRAMES=path, CLOCK_IN_FRAME_COUNT=str(START + count),
                   COLORTERM="truecolor", **{k: str(v) for k, v in env.items()})
        subprocess.run(["cargo", "test", "-q", "--", "--ignored", "--exact", f"tui::demo::{test}"],
                       cwd=REPO, env=env, check=True, stdout=subprocess.DEVNULL)
        with open(path, encoding="utf-8") as f:
            text = f.read()
    chunks = [c.rstrip("\n").split("\n") for c in text.split("\f") if c.strip()]
    return chunks[START:]


def save(images, path, loops):
    """Lossless animated WebP that plays `loops` times and holds the last frame."""
    images[0].save(path, save_all=True, append_images=images[1:], duration=FRAME_MS,
                   loop=loops, lossless=True, method=4, background=(0, 0, 0, 0))
    print(f"{path} {images[0].size[0]}x{images[0].size[1]}, {os.path.getsize(path) // 1024} KB")
