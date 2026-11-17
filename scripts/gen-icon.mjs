// 生成 1024x1024 应用图标：暗底 + 霓虹青「边芽」叶片斜刃 + 微光
// 零依赖：手写 PNG 编码（zlib 内置）
import zlib from "node:zlib";
import fs from "node:fs";

const S = 1024;
const px = new Uint8Array(S * S * 4);

// ---------- 工具 ----------
const clamp01 = (v) => Math.max(0, Math.min(1, v));
const smooth = (a, b, v) => {
  const t = clamp01((v - a) / (b - a));
  return t * t * (3 - 2 * t);
};
const lerp = (a, b, t) => a + (b - a) * t;

// ---------- 形状 ----------
// 圆角矩形范围（图标底盘）
const R = 230;
function roundedSDF(x, y) {
  const half = S / 2;
  const qx = Math.abs(x - half) - (half - R);
  const qy = Math.abs(y - half) - (half - R);
  const ox = Math.max(qx, 0);
  const oy = Math.max(qy, 0);
  return Math.hypot(ox, oy) + Math.min(Math.max(qx, qy), 0) - R;
}

// 主叶片：二次贝塞尔（从右下边缘向左上扬起的新芽）
const P0 = { x: 700, y: 880 };
const P1 = { x: 180, y: 760 };
const P2 = { x: 330, y: 200 };
const SAMPLES = [];
for (let i = 0; i <= 220; i++) {
  const t = i / 220;
  const mt = 1 - t;
  SAMPLES.push({
    x: mt * mt * P0.x + 2 * mt * t * P1.x + t * t * P2.x,
    y: mt * mt * P0.y + 2 * mt * t * P1.y + t * t * P2.y,
  });
}
function distToBlade(x, y) {
  let d = Infinity;
  for (const p of SAMPLES) {
    const dd = (p.x - x) * (p.x - x) + (p.y - y) * (p.y - y);
    if (dd < d) d = dd;
  }
  return Math.sqrt(d);
}

// 副叶（小芽）：短弧
const Q0 = { x: 660, y: 840 };
const Q1 = { x: 620, y: 560 };
const Q2 = { x: 800, y: 400 };
const SAMPLES2 = [];
for (let i = 0; i <= 120; i++) {
  const t = i / 120;
  const mt = 1 - t;
  SAMPLES2.push({
    x: mt * mt * Q0.x + 2 * mt * t * Q1.x + t * t * Q2.x,
    y: mt * mt * Q0.y + 2 * mt * t * Q1.y + t * t * Q2.y,
  });
}
function distToBlade2(x, y) {
  let d = Infinity;
  for (const p of SAMPLES2) {
    const dd = (p.x - x) * (p.x - x) + (p.y - y) * (p.y - y);
    if (dd < d) d = dd;
  }
  return Math.sqrt(d);
}

// 芽尖光点
const DOT = { x: 335, y: 195, r: 36 };

// ---------- 逐像素 ----------
for (let y = 0; y < S; y++) {
  for (let x = 0; x < S; x++) {
    const idx = (y * S + x) * 4;
    const rd = roundedSDF(x, y);
    if (rd > 2) {
      px[idx + 3] = 0; // 圆角外透明
      continue;
    }

    // 底盘：深空色垂直渐变 + 左上微亮
    const t = y / S;
    let r = lerp(13, 8, t);
    let g = lerp(19, 13, t);
    let b = lerp(26, 18, t);
    const sheen = smooth(500, 0, Math.hypot(x - 300, y - 260)) * 0.06;
    r += sheen * 40;
    g += sheen * 70;
    b += sheen * 80;

    // 边缘描一圈极淡的青光
    const edge = 1 - smooth(0, 26, Math.abs(rd + 8));
    r += edge * 22;
    g += edge * 60;
    b += edge * 58;

    // 主叶：霓虹青，芯亮缘晕
    const d1 = distToBlade(x, y);
    const core1 = 1 - smooth(20, 38, d1); // 实芯
    const glow1 = Math.pow(1 - clamp01(d1 / 85), 2.2) * 0.55;
    const v1 = Math.min(1, core1 + glow1);
    r = lerp(r, 0, v1 * 0.1);
    g = lerp(g, 235, v1);
    b = lerp(b, 214, v1);

    // 副叶：细一号但保持可读亮度
    const d2 = distToBlade2(x, y);
    const core2 = 1 - smooth(10, 24, d2);
    const glow2 = Math.pow(1 - clamp01(d2 / 70), 2.4) * 0.45;
    const v2 = Math.min(1, core2 + glow2) * 0.92;
    r = lerp(r, 40, v2 * 0.35);
    g = lerp(g, 205, v2);
    b = lerp(b, 200, v2);

    // 芽尖光点
    const d3 = Math.hypot(x - DOT.x, y - DOT.y);
    const v3 = Math.min(1, (1 - smooth(DOT.r * 0.4, DOT.r, d3)) + Math.pow(1 - clamp01(d3 / 70), 2) * 0.5);
    r = lerp(r, 160, v3 * 0.5);
    g = lerp(g, 255, v3);
    b = lerp(b, 240, v3);

    px[idx] = Math.min(255, Math.round(r));
    px[idx + 1] = Math.min(255, Math.round(g));
    px[idx + 2] = Math.min(255, Math.round(b));
    px[idx + 3] = 255;
  }
}

// ---------- PNG 编码 ----------
const crcTable = [];
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  crcTable[n] = c >>> 0;
}
function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}

const raw = Buffer.alloc(S * (S * 4 + 1), 0);
for (let y = 0; y < S; y++) {
  raw[y * (S * 4 + 1)] = 0; // filter: none
  Buffer.from(px.buffer, y * S * 4, S * 4).copy(raw, y * (S * 4 + 1) + 1);
}
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);
fs.mkdirSync("scripts/out", { recursive: true });
fs.writeFileSync("scripts/out/icon-src.png", png);
console.log("icon written:", png.length, "bytes");
