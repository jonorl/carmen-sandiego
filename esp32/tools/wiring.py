"""Draws docs/wiring.svg, the wiring diagram, from the README's wiring table
(kept in step with it by hand): python3 tools/wiring.py"""
import sys

out = sys.argv[1] if len(sys.argv) > 1 else "docs/wiring.svg"
W, H = 960, 750
DX, EX = 250, 700          # display pins' x, ESP32 pins' x
TOP, STEP = 150, 30
POWER, GROUND, SCREEN, TOUCH, SOUND = '#d62828', '#222222', '#1d4ed8', '#7c3aed', '#c2410c'

disp = ['VCC', 'GND', 'CS', 'RESET', 'DC', 'SDI (MOSI)', 'SCK', 'LED', 'SDO (MISO)', 'T_CLK', 'T_CS', 'T_DIN', 'T_DO', 'T_IRQ']
esp = ['5V', 'GND', 'GPIO 10', 'GPIO 8', 'GPIO 9', 'GPIO 11', 'GPIO 12', '3V3', 'GPIO 7', 'GPIO 13', 'GPIO 6', 'GPIO 4']
dy = {p: TOP + i * STEP for i, p in enumerate(disp)}
ey = {p: TOP + i * STEP for i, p in enumerate(esp)}

s = []
add = s.append
add(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" font-family="Helvetica, Arial, sans-serif" font-size="13">')
add(f'<rect width="{W}" height="{H}" fill="#ffffff"/>')
add(f'<text x="{W/2}" y="30" text-anchor="middle" font-size="20" font-weight="bold" fill="#111">carmen-esp32 wiring</text>')

def wire(points, colour, width=2.5):
    d = ' '.join(f'{x},{y}' for x, y in points)
    add(f'<polyline points="{d}" fill="none" stroke="{colour}" stroke-width="{width}" stroke-linejoin="round"/>')

def dot(x, y, colour):
    add(f'<circle cx="{x}" cy="{y}" r="4.5" fill="{colour}"/>')

def ground(x, y):
    # A ground symbol hanging below (x, y).
    add(f'<g stroke="{GROUND}" stroke-width="2"><line x1="{x}" y1="{y}" x2="{x}" y2="{y+10}"/>'
        f'<line x1="{x-9}" y1="{y+10}" x2="{x+9}" y2="{y+10}"/><line x1="{x-6}" y1="{y+14}" x2="{x+6}" y2="{y+14}"/>'
        f'<line x1="{x-3}" y1="{y+18}" x2="{x+3}" y2="{y+18}"/></g>')

# Straight across: same row on both sides.
straight = {'CS': ('GPIO 10', SCREEN), 'RESET': ('GPIO 8', SCREEN), 'DC': ('GPIO 9', SCREEN),
            'SDI (MOSI)': ('GPIO 11', SCREEN), 'SCK': ('GPIO 12', SCREEN), 'LED': ('3V3', POWER)}
for p, (e, c) in straight.items():
    wire([(DX, dy[p]), (EX, ey[e])], c)
# Touch: down a channel of its own, joining the screen's line where they share a pin.
bent = [('T_DIN', 'GPIO 11', 370, True), ('T_CLK', 'GPIO 12', 400, True), ('T_CS', 'GPIO 7', 430, False),
        ('T_DO', 'GPIO 13', 520, False), ('T_IRQ', 'GPIO 6', 560, False)]
for p, e, cx, shared in bent:
    wire([(DX, dy[p]), (cx, dy[p]), (cx, ey[e])] + ([] if shared else [(EX, ey[e])]), TOUCH)
    if shared:
        dot(cx, ey[e], TOUCH)
# Power: the module's 5 V feeds the screen's VCC and the board's 5V pin.
mod = (300, 52, 360, 62)
x0, y0, w, h = mod
add(f'<rect x="{x0}" y="{y0}" width="{w}" height="{h}" rx="6" fill="#fde68a" stroke="#92400e" stroke-width="2"/>')
add(f'<text x="{x0+w/2}" y="{y0+24}" text-anchor="middle" font-weight="bold" fill="#111">Breadboard power module (MB102)</text>')
add(f'<text x="{x0+w/2}" y="{y0+44}" text-anchor="middle" fill="#333">A 5 V USB supply in its USB socket; rail jumpers on 5V</text>')
vin = TOP
wire([(DX, dy['VCC']), (EX, ey['5V'])], POWER)
wire([(x0 + w/2, y0 + h), (x0 + w/2, vin)], POWER)
dot(x0 + w/2, vin, POWER)
add(f'<text x="{x0+w/2+8}" y="{vin-8}" fill="{POWER}" font-weight="bold">5V</text>')
# Grounds: one symbol each, all joined (the module's too).
ground(DX + 22, dy['GND'])
wire([(DX, dy['GND']), (DX + 22, dy['GND'])], GROUND)
ground(EX - 22, ey['GND'])
wire([(EX - 22, ey['GND']), (EX, ey['GND'])], GROUND)
ground(x0 + w - 40, y0 + h)
add(f'<text x="{x0+w-26}" y="{y0+h+16}" fill="{GROUND}">GND</text>')

# The piezo, below, to GPIO 4 and ground.
px, py = 600, 600
add(f'<circle cx="{px}" cy="{py}" r="24" fill="#333" stroke="#000" stroke-width="2"/><circle cx="{px}" cy="{py}" r="6" fill="#facc15"/>')
add(f'<text x="{px}" y="{py+42}" text-anchor="middle" fill="#111">Piezo (either way round)</text>')
wire([(px + 24, py), (650, py), (650, ey['GPIO 4']), (EX, ey['GPIO 4'])], SOUND)
wire([(px - 24, py), (560, py)], GROUND)
ground(560, py)

# The display module: its header, in the order it's printed.
add(f'<rect x="40" y="{TOP-40}" width="{DX-40}" height="{STEP*len(disp)+50}" rx="6" fill="#fee2e2" stroke="#991b1b" stroke-width="2"/>')
add(f'<text x="{(40+DX)/2}" y="{TOP-18}" text-anchor="middle" font-weight="bold" fill="#111">2.8" ILI9341 + XPT2046</text>')
for p, y in dy.items():
    add(f'<circle cx="{DX}" cy="{y}" r="5" fill="#fff" stroke="#111" stroke-width="2"/>')
    add(f'<text x="{DX-12}" y="{y+4}" text-anchor="end" fill="#111">{p}</text>')
add(f'<text x="{DX+14}" y="{dy["SDO (MISO)"]+4}" fill="#666" font-style="italic">not connected</text>')
add(f'<text x="58" y="{TOP+STEP*len(disp)+2}" fill="#991b1b">Leave jumper J1 open</text>')

# The ESP32-S3 board.
add(f'<rect x="{EX}" y="{TOP-40}" width="210" height="{STEP*len(esp)+50}" rx="6" fill="#e0e7ff" stroke="#1e3a8a" stroke-width="2"/>')
add(f'<text x="{EX+105}" y="{TOP-18}" text-anchor="middle" font-weight="bold" fill="#111">ESP32-S3 DevKitC-1</text>')
for p, y in ey.items():
    add(f'<circle cx="{EX}" cy="{y}" r="5" fill="#fff" stroke="#111" stroke-width="2"/>')
    add(f'<text x="{EX+12}" y="{y+4}" fill="#111">{p}</text>')
add(f'<text x="{EX+12}" y="{TOP+STEP*len(esp)+2}" fill="#1e3a8a">5V pin: power in only</text>')

# Key.
ky = 670
for i, (name, c) in enumerate([('Power', POWER), ('Ground', GROUND), ('Screen (SPI2)', SCREEN), ('Touch', TOUCH), ('Sound', SOUND)]):
    y = ky + i * 16
    add(f'<line x1="60" y1="{y}" x2="90" y2="{y}" stroke="{c}" stroke-width="3"/><text x="98" y="{y+4}" fill="#111">{name}</text>')
add(f'<text x="300" y="{ky+4}" fill="#333">A dot joins wires; lines that just cross aren\'t joined.</text>')
add(f'<text x="300" y="{ky+20}" fill="#333">No soldering: every joint is a jumper wire on the breadboard.</text>')
add('</svg>')
open(out, 'w').write('\n'.join(s) + '\n')
