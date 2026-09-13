"""Generate the 米游抢码器 (mhy-QRscanner) app icon.

Design: Miyoushe-cyan gradient square (#96EEFF -> #66E0FF), a 5x5 grid of large
QR pixels in the Miyoushe outline blue (#19A3FF), and a yellow scan beam
(#FACC15) across the middle for the 抢 (race) cue. Drawn at 1024 and exported
as a multi-size .ico to both consumers:

  - app/windows/runner/resources/app_icon.ico  (title bar / taskbar / exe)
  - app/assets/app_icon.ico                    (tray icon)

    python app/tool/make_icon.py   # from anywhere; paths are repo-relative
"""
import pathlib

from PIL import Image, ImageDraw

S = 1024
MARGIN = 44
RADIUS = 216
CYAN = (102, 224, 255)
CYAN_LIGHT = (150, 238, 255)
BLUE = (25, 163, 255)
YELLOW = (250, 204, 21)
ROOT = pathlib.Path(__file__).resolve().parents[2]


def gradient(size, top, bottom):
    img = Image.new('RGB', (size, size))
    px = img.load()
    for y in range(size):
        t = y / (size - 1)
        c = tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
        for x in range(size):
            px[x, y] = c
    return img.convert('RGBA')


def finish(img):
    mask = Image.new('L', (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [MARGIN, MARGIN, S - MARGIN, S - MARGIN], radius=RADIUS, fill=255)
    out = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)
    ImageDraw.Draw(out).rounded_rectangle(
        [MARGIN, MARGIN, S - MARGIN, S - MARGIN], radius=RADIUS,
        outline=(255, 255, 255, 60), width=4)
    return out.resize((256, 256), Image.LANCZOS)


def build():
    img = gradient(S, CYAN_LIGHT, CYAN)
    layer = Image.new('RGBA', (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    d.polygon([(S * 0.55, 0), (S, 0), (S, S * 0.6), (S * 0.2, 0)], fill=(255, 255, 255, 40))
    img = Image.alpha_composite(img, layer)

    d = ImageDraw.Draw(img)
    n = 5
    grid = S - 2 * (MARGIN + 150)
    mod = grid / n
    pattern = [[1, 0, 1, 1, 0], [1, 0, 0, 1, 1], [0, 1, 1, 0, 1],
               [1, 1, 0, 1, 0], [1, 0, 1, 0, 1]]
    x0 = y0 = MARGIN + 150
    for gy in range(n):
        for gx in range(n):
            if pattern[gy][gx]:
                px, py = x0 + gx * mod, y0 + gy * mod
                d.rounded_rectangle([px + mod * 0.10, py + mod * 0.10,
                                     px + mod * 0.90, py + mod * 0.90],
                                    radius=mod * 0.22, fill=BLUE)

    yc = S / 2
    halo = Image.new('RGBA', img.size, (0, 0, 0, 0))
    ImageDraw.Draw(halo).rounded_rectangle(
        [MARGIN + 60, yc - 76, S - MARGIN - 60, yc + 76], radius=76,
        fill=YELLOW + (70,))
    img = Image.alpha_composite(img, halo)
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([MARGIN + 70, yc - 28, S - MARGIN - 70, yc + 28], radius=28,
                        fill=YELLOW + (235,))
    return finish(img)


def main():
    master = build()
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    master.save(ROOT / 'app/windows/runner/resources/app_icon.ico', format='ICO', sizes=ico_sizes)
    master.save(ROOT / 'app/assets/app_icon.ico', format='ICO', sizes=ico_sizes)
    print('icons written: app/windows/runner/resources/app_icon.ico, app/assets/app_icon.ico')


if __name__ == '__main__':
    main()
