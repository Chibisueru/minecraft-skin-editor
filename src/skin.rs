//! Skin texture model, player geometry and the software ray caster that both
//! renders the 3D preview and resolves which texel is under the cursor.

use std::path::Path;

pub const TEX: usize = 64;

pub type Rgba = [u8; 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// 4 pixel wide arms ("Steve").
    Classic,
    /// 3 pixel wide arms ("Alex").
    Slim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Base,
    Overlay,
}

pub const PART_NAMES: [&str; 6] = [
    "Head",
    "Body",
    "Right arm",
    "Left arm",
    "Right leg",
    "Left leg",
];

/// Which body parts are shown, per layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visibility {
    pub base: [bool; 6],
    pub overlay: [bool; 6],
}

impl Default for Visibility {
    fn default() -> Self {
        Self {
            base: [true; 6],
            overlay: [true; 6],
        }
    }
}

impl Visibility {
    pub fn get(&self, layer: Layer, part: usize) -> bool {
        match layer {
            Layer::Base => self.base[part],
            Layer::Overlay => self.overlay[part],
        }
    }

    pub fn layer_mut(&mut self, layer: Layer) -> &mut [bool; 6] {
        match layer {
            Layer::Base => &mut self.base,
            Layer::Overlay => &mut self.overlay,
        }
    }
}

/// A rectangle of texels: x, y, width, height.
pub type TexRect = (usize, usize, usize, usize);

/// One cuboid of the player model and where its texture lives.
#[derive(Clone, Copy, Debug)]
pub struct BoxDef {
    pub part: usize,
    pub layer: Layer,
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Top-left corner of the unwrapped box in the texture.
    pub uv: (usize, usize),
    /// Width (x), height (y) and depth (z) in texels.
    pub dims: (usize, usize, usize),
}

/// The twelve boxes (six parts, two layers) of a player model.
///
/// Model space: y is up, feet at y = 0, the player looks towards +z, so the
/// player's right arm sits at negative x.
pub fn boxes(model: Model) -> Vec<BoxDef> {
    let arm = match model {
        Model::Classic => 4,
        Model::Slim => 3,
    };
    let a = arm as f32;
    // (min, max, dims, base uv, overlay uv)
    let parts: [([f32; 3], [f32; 3], (usize, usize, usize), (usize, usize), (usize, usize)); 6] = [
        ([-4.0, 24.0, -4.0], [4.0, 32.0, 4.0], (8, 8, 8), (0, 0), (32, 0)),
        ([-4.0, 12.0, -2.0], [4.0, 24.0, 2.0], (8, 12, 4), (16, 16), (16, 32)),
        ([-4.0 - a, 12.0, -2.0], [-4.0, 24.0, 2.0], (arm, 12, 4), (40, 16), (40, 32)),
        ([4.0, 12.0, -2.0], [4.0 + a, 24.0, 2.0], (arm, 12, 4), (32, 48), (48, 48)),
        ([-4.0, 0.0, -2.0], [0.0, 12.0, 2.0], (4, 12, 4), (0, 16), (0, 32)),
        ([0.0, 0.0, -2.0], [4.0, 12.0, 2.0], (4, 12, 4), (16, 48), (0, 48)),
    ];

    let mut out = Vec::with_capacity(12);
    for (part, (min, max, dims, uv_base, uv_overlay)) in parts.into_iter().enumerate() {
        out.push(BoxDef {
            part,
            layer: Layer::Base,
            min,
            max,
            uv: uv_base,
            dims,
        });
        let grow = if part == 0 { 0.5 } else { 0.25 };
        out.push(BoxDef {
            part,
            layer: Layer::Overlay,
            min: [min[0] - grow, min[1] - grow, min[2] - grow],
            max: [max[0] + grow, max[1] + grow, max[2] + grow],
            uv: uv_overlay,
            dims,
        });
    }
    out
}

/// A 64x64 RGBA skin texture.
#[derive(Clone, PartialEq, Eq)]
pub struct Skin {
    pub px: Vec<Rgba>,
}

impl Skin {
    pub fn blank() -> Self {
        Self {
            px: vec![[0; 4]; TEX * TEX],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgba {
        self.px[y * TEX + x]
    }

    pub fn set(&mut self, x: usize, y: usize, c: Rgba) {
        self.px[y * TEX + x] = c;
    }

    fn fill(&mut self, (x, y, w, h): TexRect, c: Rgba) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    /// The built-in character a new document starts with, and its arm model.
    pub fn default_character() -> (Self, Model) {
        let img = image::load_from_memory_with_format(
            &crate::default_skin::PNG,
            image::ImageFormat::Png,
        )
        .expect("the built-in skin is a valid PNG")
        .to_rgba8();
        let mut s = Self::blank();
        for (x, y, p) in img.enumerate_pixels() {
            s.set(x as usize, y as usize, p.0);
        }
        let model = s.guess_model();
        (s, model)
    }

    /// Slim skins leave the fourth arm column unused.
    fn guess_model(&self) -> Model {
        let slim = (20..32).all(|y| (54..56).all(|x| self.get(x, y)[3] == 0))
            && (20..32).any(|y| (44..54).any(|x| self.get(x, y)[3] != 0));
        if slim {
            Model::Slim
        } else {
            Model::Classic
        }
    }

    /// A plain, left-right symmetric character for the tests.
    #[cfg(test)]
    fn placeholder(model: Model) -> Self {
        const SKIN: Rgba = [224, 172, 136, 255];
        const HAIR: Rgba = [74, 48, 32, 255];
        const SHIRT: Rgba = [38, 166, 154, 255];
        const PANTS: Rgba = [57, 73, 171, 255];
        const SHOES: Rgba = [66, 66, 66, 255];
        const WHITE: Rgba = [250, 250, 250, 255];
        const EYE: Rgba = [48, 63, 120, 255];
        const MOUTH: Rgba = [158, 94, 80, 255];

        let mut s = Self::blank();
        for b in boxes(model).iter().filter(|b| b.layer == Layer::Base) {
            let (u, v) = b.uv;
            let (w, h, d) = b.dims;
            let caps = (u + d, v, 2 * w, d);
            let sides = (u, v + d, 2 * d + 2 * w, h);
            match b.part {
                0 => {
                    s.fill(caps, SKIN);
                    s.fill(sides, SKIN);
                    s.fill((u + d, v, w, d), HAIR); // top of the head
                    s.fill((u, v + d, 2 * d + 2 * w, 2), HAIR); // fringe all around
                    s.fill((u + d + w, v + d, d + w, 5), HAIR); // left side + back
                    s.fill((u, v + d, d, 5), HAIR); // right side
                    let (fx, fy) = (u + d, v + d);
                    s.set(fx + 1, fy + 4, WHITE);
                    s.set(fx + 2, fy + 4, EYE);
                    s.set(fx + 5, fy + 4, EYE);
                    s.set(fx + 6, fy + 4, WHITE);
                    s.set(fx + 3, fy + 6, MOUTH);
                    s.set(fx + 4, fy + 6, MOUTH);
                }
                1 => {
                    s.fill(caps, SHIRT);
                    s.fill(sides, SHIRT);
                }
                2 | 3 => {
                    s.fill(caps, SKIN);
                    s.fill(sides, SKIN);
                    s.fill((u + d, v, w, d), SHIRT); // shoulder
                    s.fill((u, v + d, 2 * d + 2 * w, 4), SHIRT); // sleeve
                }
                _ => {
                    s.fill(caps, PANTS);
                    s.fill(sides, PANTS);
                    s.fill((u + d + w, v, w, d), SHOES); // sole
                    s.fill((u, v + d + h - 2, 2 * d + 2 * w, 2), SHOES);
                }
            }
        }
        s
    }

    /// Loads a 64x64 skin, or a legacy 64x32 one which is upgraded in memory.
    /// Returns the skin and a guess at which arm model it was drawn for.
    pub fn load(path: &Path) -> Result<(Self, Model), String> {
        let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
        let (w, h) = img.dimensions();
        if w != 64 || (h != 64 && h != 32) {
            return Err(format!(
                "{w}x{h} is not a Minecraft skin size (expected 64x64 or 64x32)"
            ));
        }
        let mut s = Self::blank();
        for (x, y, p) in img.enumerate_pixels() {
            s.set(x as usize, y as usize, p.0);
        }
        if h == 32 {
            s.upgrade_legacy();
            return Ok((s, Model::Classic));
        }
        let model = s.guess_model();
        Ok((s, model))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let mut raw = Vec::with_capacity(TEX * TEX * 4);
        for p in &self.px {
            raw.extend_from_slice(p);
        }
        image::RgbaImage::from_raw(TEX as u32, TEX as u32, raw)
            .ok_or_else(|| "invalid image buffer".to_string())?
            .save_with_format(path, image::ImageFormat::Png)
            .map_err(|e| e.to_string())
    }

    /// 64x32 skins have no left limbs: mirror them from the right ones.
    fn upgrade_legacy(&mut self) {
        self.mirror_limb((0, 16), (16, 48));
        self.mirror_limb((40, 16), (32, 48));
    }

    fn mirror_limb(&mut self, (su, sv): (usize, usize), (du, dv): (usize, usize)) {
        let (w, h, d) = (4, 12, 4);
        // (source rect origin, destination rect origin, width, height)
        let faces = [
            ((su + d, sv), (du + d, dv), w, d),                 // top
            ((su + d + w, sv), (du + d + w, dv), w, d),         // bottom
            ((su, sv + d), (du + d + w, dv + d), d, h),         // outer -> outer
            ((su + d + w, sv + d), (du, dv + d), d, h),         // inner -> inner
            ((su + d, sv + d), (du + d, dv + d), w, h),         // front
            ((su + 2 * d + w, sv + d), (du + 2 * d + w, dv + d), w, h), // back
        ];
        for ((sx, sy), (dx, dy), fw, fh) in faces {
            for y in 0..fh {
                for x in 0..fw {
                    let c = self.get(sx + x, sy + y);
                    self.set(dx + fw - 1 - x, dy + y, c);
                }
            }
        }
    }

    /// Flood fills the connected area of equal colour, staying on one face.
    pub fn flood_fill(&mut self, x: usize, y: usize, (rx, ry, rw, rh): TexRect, c: Rgba) {
        let from = self.get(x, y);
        if from == c {
            return;
        }
        let mut stack = vec![(x, y)];
        while let Some((x, y)) = stack.pop() {
            if self.get(x, y) != from {
                continue;
            }
            self.set(x, y, c);
            if x > rx {
                stack.push((x - 1, y));
            }
            if x + 1 < rx + rw {
                stack.push((x + 1, y));
            }
            if y > ry {
                stack.push((x, y - 1));
            }
            if y + 1 < ry + rh {
                stack.push((x, y + 1));
            }
        }
    }

    /// Clears every texel belonging to the overlay layer.
    pub fn clear_overlay(&mut self) {
        // Arm width does not matter here: the classic rects cover the slim ones.
        for b in boxes(Model::Classic).iter().filter(|b| b.layer == Layer::Overlay) {
            let (u, v) = b.uv;
            let (w, h, d) = b.dims;
            self.fill((u, v, 2 * d + 2 * w, d + h), [0; 4]);
        }
    }
}

// ---------------------------------------------------------------- camera ---

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    /// Shift of the model on screen, as a fraction of the viewport height.
    pub pan: (f32, f32),
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.5,
            pitch: 0.2,
            dist: 85.0,
            pan: (0.0, 0.0),
        }
    }
}

const TARGET: [f32; 3] = [0.0, 16.0, 0.0];
const TAN_HALF_FOV: f32 = 0.25;

impl Camera {
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.01;
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.55, 1.55);
    }

    /// Moves the model by a pointer delta in a viewport `height` pixels tall.
    pub fn pan(&mut self, dx: f32, dy: f32, height: f32) {
        self.pan.0 = (self.pan.0 + dx / height).clamp(-1.5, 1.5);
        self.pan.1 = (self.pan.1 + dy / height).clamp(-1.0, 1.0);
    }

    pub fn zoom(&mut self, steps: f32) {
        self.dist = (self.dist * 0.9f32.powf(steps)).clamp(25.0, 300.0);
    }

    fn basis(&self) -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let back = [sy * cp, sp, cy * cp];
        let forward = [-back[0], -back[1], -back[2]];
        let right = [cy, 0.0, -sy];
        let up = cross(right, forward);
        let origin = [
            TARGET[0] + back[0] * self.dist,
            TARGET[1] + back[1] * self.dist,
            TARGET[2] + back[2] * self.dist,
        ];
        (origin, forward, right, up)
    }

    /// Ray through the viewport position `(x, y)` of a `w` by `h` viewport.
    pub fn ray(&self, x: f32, y: f32, w: f32, h: f32) -> Ray {
        let (origin, f, r, u) = self.basis();
        let k = TAN_HALF_FOV / (h * 0.5);
        let sx = (x - w * 0.5 - self.pan.0 * h) * k;
        let sy = (h * 0.5 - y + self.pan.1 * h) * k;
        Ray {
            o: origin,
            d: [
                f[0] + r[0] * sx + u[0] * sy,
                f[1] + r[1] * sx + u[1] * sy,
                f[2] + r[2] * sx + u[2] * sy,
            ],
        }
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub o: [f32; 3],
    pub d: [f32; 3],
}

// ------------------------------------------------------------ ray casting ---

/// Entry and exit of a ray through a box: (t, axis, hit the max side?).
type Crossing = (f32, usize, bool);

fn intersect(ray: &Ray, min: &[f32; 3], max: &[f32; 3]) -> Option<(Crossing, Crossing)> {
    let mut t_in = f32::NEG_INFINITY;
    let mut t_out = f32::INFINITY;
    let (mut a_in, mut a_out) = (0, 0);
    for a in 0..3 {
        let d = ray.d[a];
        if d.abs() < 1e-9 {
            if ray.o[a] < min[a] || ray.o[a] > max[a] {
                return None;
            }
            continue;
        }
        let inv = 1.0 / d;
        let (near, far) = if d > 0.0 {
            ((min[a] - ray.o[a]) * inv, (max[a] - ray.o[a]) * inv)
        } else {
            ((max[a] - ray.o[a]) * inv, (min[a] - ray.o[a]) * inv)
        };
        if near > t_in {
            t_in = near;
            a_in = a;
        }
        if far < t_out {
            t_out = far;
            a_out = a;
        }
    }
    if t_in > t_out || t_out < 0.0 {
        return None;
    }
    Some((
        (t_in, a_in, ray.d[a_in] < 0.0),
        (t_out, a_out, ray.d[a_out] > 0.0),
    ))
}

/// A texel on a box face.
#[derive(Clone, Copy, Debug)]
pub struct Texel {
    pub x: usize,
    pub y: usize,
    /// The texture rectangle of the face this texel belongs to.
    pub face: TexRect,
    shade: f32,
    /// Distance to the nearest texel border, in texels.
    edge: f32,
}

fn face_texel(b: &BoxDef, ray: &Ray, (t, axis, max_side): Crossing) -> Texel {
    let mut n = [0.0f32; 3];
    for a in 0..3 {
        let p = ray.o[a] + ray.d[a] * t;
        n[a] = ((p - b.min[a]) / (b.max[a] - b.min[a])).clamp(0.0, 0.999_99);
    }
    let (u, v) = b.uv;
    let (w, h, d) = b.dims;
    // Texture rect of the face, the position inside it (0..1) and brightness.
    let (face, fu, fv, shade) = match (axis, max_side) {
        (2, true) => ((u + d, v + d, w, h), n[0], 1.0 - n[1], 0.92),
        (2, false) => ((u + 2 * d + w, v + d, w, h), 1.0 - n[0], 1.0 - n[1], 0.72),
        (0, false) => ((u, v + d, d, h), n[2], 1.0 - n[1], 0.8),
        (0, true) => ((u + d + w, v + d, d, h), 1.0 - n[2], 1.0 - n[1], 0.8),
        (_, true) => ((u + d, v, w, d), n[0], n[2], 1.0),
        (_, false) => ((u + d + w, v, w, d), n[0], n[2], 0.6),
    };
    // `1.0 - n` reaches exactly 1.0 on a box edge; keep it inside the face.
    let (fu, fv) = (fu.min(0.999_99), fv.min(0.999_99));
    let (su, sv) = (fu * face.2 as f32, fv * face.3 as f32);
    let (fx, fy) = (su.fract(), sv.fract());
    Texel {
        x: face.0 + su as usize,
        y: face.1 + sv as usize,
        face,
        shade,
        edge: fx.min(1.0 - fx).min(fy).min(1.0 - fy),
    }
}

pub struct Scene<'a> {
    pub skin: &'a Skin,
    pub boxes: &'a [BoxDef],
    pub vis: &'a Visibility,
}

impl Scene<'_> {
    fn visible_boxes(&self) -> impl Iterator<Item = &BoxDef> {
        self.boxes.iter().filter(|b| self.vis.get(b.layer, b.part))
    }

    /// The nearest outward-facing surface among the shown boxes, whatever its
    /// colour. This is what the painting tools draw on: the overlay where it
    /// is shown, the base layer where the overlay is hidden.
    pub fn pick_surface(&self, ray: &Ray) -> Option<Texel> {
        let mut best: Option<(f32, Texel)> = None;
        for b in self.visible_boxes() {
            if let Some((entry, _)) = intersect(ray, &b.min, &b.max) {
                if entry.0 > 0.0 && best.map_or(true, |(t, _)| entry.0 < t) {
                    best = Some((entry.0, face_texel(b, ray, entry)));
                }
            }
        }
        best.map(|(_, texel)| texel)
    }

    /// The nearest texel that is actually visible (not transparent).
    pub fn pick_visible(&self, ray: &Ray) -> Option<Texel> {
        let mut best: Option<(f32, Texel)> = None;
        for b in self.visible_boxes() {
            let Some((entry, exit)) = intersect(ray, &b.min, &b.max) else {
                continue;
            };
            for crossing in [entry, exit] {
                if crossing.0 <= 0.0 || best.map_or(false, |(t, _)| crossing.0 >= t) {
                    continue;
                }
                let texel = face_texel(b, ray, crossing);
                if self.skin.get(texel.x, texel.y)[3] > 0 {
                    best = Some((crossing.0, texel));
                }
            }
        }
        best.map(|(_, texel)| texel)
    }

    /// Colour seen along a ray, as straight-alpha RGBA.
    fn shade(&self, ray: &Ray, grid: bool) -> Rgba {
        // (distance, premultiplied rgb, alpha) of every surface along the ray.
        let mut hits = [(0.0f32, [0.0f32; 3], 0.0f32); 24];
        let mut count = 0;
        for b in self.visible_boxes() {
            let Some((entry, exit)) = intersect(ray, &b.min, &b.max) else {
                continue;
            };
            for crossing in [entry, exit] {
                if crossing.0 <= 0.0 {
                    continue;
                }
                let texel = face_texel(b, ray, crossing);
                let c = self.skin.get(texel.x, texel.y);
                if c[3] == 0 {
                    continue;
                }
                let mut light = texel.shade;
                if grid && texel.edge < 0.04 {
                    light *= 0.8;
                }
                let alpha = c[3] as f32 / 255.0;
                let k = light * alpha;
                hits[count] = (
                    crossing.0,
                    [c[0] as f32 * k, c[1] as f32 * k, c[2] as f32 * k],
                    alpha,
                );
                count += 1;
            }
        }
        if count == 0 {
            return [0; 4];
        }
        let hits = &mut hits[..count];
        hits.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

        let mut rgb = [0.0f32; 3];
        let mut alpha = 0.0f32;
        for (_, c, a) in hits.iter() {
            let rest = 1.0 - alpha;
            rgb[0] += c[0] * rest;
            rgb[1] += c[1] * rest;
            rgb[2] += c[2] * rest;
            alpha += a * rest;
            if alpha > 0.995 {
                break;
            }
        }
        [
            (rgb[0] / alpha).round().min(255.0) as u8,
            (rgb[1] / alpha).round().min(255.0) as u8,
            (rgb[2] / alpha).round().min(255.0) as u8,
            (alpha * 255.0).round() as u8,
        ]
    }

    /// Renders the model into a `w` by `h` straight-alpha RGBA buffer.
    pub fn render(&self, cam: &Camera, w: usize, h: usize, grid: bool) -> Vec<u8> {
        let mut buf = vec![0u8; w * h * 4];
        if w == 0 || h == 0 {
            return buf;
        }
        // Everything lives inside this box; most rays miss it and exit early.
        let (bound_min, bound_max) = ([-8.6, -0.6, -4.6], [8.6, 32.6, 4.6]);
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
        let rows_per_chunk = h.div_ceil(threads).max(1);

        std::thread::scope(|scope| {
            for (chunk_index, chunk) in buf.chunks_mut(rows_per_chunk * w * 4).enumerate() {
                scope.spawn(move || {
                    let first_row = chunk_index * rows_per_chunk;
                    for (i, px) in chunk.chunks_exact_mut(4).enumerate() {
                        let (x, y) = (i % w, first_row + i / w);
                        let ray = cam.ray(x as f32 + 0.5, y as f32 + 0.5, w as f32, h as f32);
                        if intersect(&ray, &bound_min, &bound_max).is_some() {
                            px.copy_from_slice(&self.shade(&ray, grid));
                        }
                    }
                });
            }
        });
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_pick(model: Model, cam: Camera, vis: Visibility) -> Texel {
        let skin = Skin::placeholder(model);
        let boxes = boxes(model);
        let scene = Scene {
            skin: &skin,
            boxes: &boxes,
            vis: &vis,
        };
        // Centre of the viewport looks at y = 16: the body.
        scene
            .pick_surface(&cam.ray(50.5, 50.5, 100.0, 100.0))
            .expect("centre ray hits the body")
    }

    #[test]
    fn base_is_painted_only_where_the_overlay_is_hidden() {
        let cam = Camera {
            yaw: 0.0,
            pitch: 0.0,
            dist: 80.0,
            pan: (0.0, 0.0),
        };
        // Overlay shown: the body's overlay front face is on top.
        let t = scene_pick(Model::Classic, cam, Visibility::default());
        assert_eq!(t.face, (20, 36, 8, 12));

        let mut vis = Visibility::default();
        vis.overlay[1] = false;
        let t = scene_pick(Model::Classic, cam, vis);
        assert_eq!(t.face, (20, 20, 8, 12));
        // y = 16 is 8 texels below the shoulders; x = 0 is the body's middle.
        assert_eq!((t.x, t.y), (24, 28));
    }

    #[test]
    fn panning_moves_the_model_with_the_pointer() {
        let mut cam = Camera {
            yaw: 0.0,
            pitch: 0.0,
            dist: 80.0,
            pan: (0.0, 0.0),
        };
        let centre = scene_pick(Model::Classic, cam, Visibility::default());
        // Drag 30 px right and 20 px down in a 100 px tall viewport.
        cam.pan(30.0, 20.0, 100.0);
        let skin = Skin::placeholder(Model::Classic);
        let boxes = boxes(Model::Classic);
        let vis = Visibility::default();
        let scene = Scene {
            skin: &skin,
            boxes: &boxes,
            vis: &vis,
        };
        let moved = scene
            .pick_surface(&cam.ray(80.5, 70.5, 100.0, 100.0))
            .expect("the same spot is now 30 px right, 20 px down");
        assert_eq!((moved.x, moved.y), (centre.x, centre.y));
    }

    #[test]
    fn back_view_hits_body_back_on_overlay() {
        let cam = Camera {
            yaw: std::f32::consts::PI,
            pitch: 0.0,
            dist: 80.0,
            pan: (0.0, 0.0),
        };
        let t = scene_pick(Model::Classic, cam, Visibility::default());
        assert_eq!(t.face, (32, 36, 8, 12));
    }

    #[test]
    fn rays_grazing_box_edges_stay_inside_the_texture() {
        let skin = Skin::placeholder(Model::Classic);
        let boxes = boxes(Model::Classic);
        let vis = Visibility::default();
        let scene = Scene {
            skin: &skin,
            boxes: &boxes,
            vis: &vis,
        };
        // Horizontal rays exactly along the feet (y = 0) and the top of the head.
        for y in [0.0, 12.0, 24.0, 32.0] {
            for x in [-8.0, -4.0, -2.0, 0.0, 2.0, 4.0, 8.0] {
                let ray = Ray {
                    o: [x, y, 50.0],
                    d: [0.0, 0.0, -1.0],
                };
                scene.shade(&ray, true);
                scene.pick_visible(&ray);
                scene.pick_surface(&ray);
            }
        }
        // And a sweep of camera angles and viewport sizes.
        for i in 0..40 {
            let cam = Camera {
                yaw: i as f32 * 0.37,
                pitch: (i as f32 * 0.21).sin() * 1.5,
                dist: 40.0 + i as f32 * 3.0,
                pan: (i as f32 * 0.02, -(i as f32) * 0.01),
            };
            scene.render(&cam, 97 + i * 13, 61 + i * 7, true);
        }
    }

    #[test]
    fn built_in_skin_decodes_to_a_full_character() {
        let (skin, _) = Skin::default_character();
        // Every base-layer face texel of the head and body is painted.
        for (x, y, w, h) in [(8, 8, 8, 8), (20, 20, 8, 12)] {
            for yy in y..y + h {
                for xx in x..x + w {
                    assert_eq!(skin.get(xx, yy)[3], 255, "hole at {xx},{yy}");
                }
            }
        }
    }

    #[test]
    fn faces_never_overlap_or_leave_the_texture() {
        for model in [Model::Classic, Model::Slim] {
            let mut seen = vec![false; TEX * TEX];
            for b in boxes(model) {
                let (u, v) = b.uv;
                let (w, h, d) = b.dims;
                let rects = [
                    (u + d, v, w, d),
                    (u + d + w, v, w, d),
                    (u, v + d, d, h),
                    (u + d, v + d, w, h),
                    (u + d + w, v + d, d, h),
                    (u + 2 * d + w, v + d, w, h),
                ];
                for (x, y, w, h) in rects {
                    for yy in y..y + h {
                        for xx in x..x + w {
                            assert!(xx < TEX && yy < TEX);
                            assert!(!seen[yy * TEX + xx], "overlap at {xx},{yy}");
                            seen[yy * TEX + xx] = true;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn render_draws_something_and_legacy_upgrade_fills_left_limbs() {
        let skin = Skin::placeholder(Model::Classic);
        let boxes = boxes(Model::Classic);
        let vis = Visibility::default();
        let scene = Scene {
            skin: &skin,
            boxes: &boxes,
            vis: &vis,
        };
        let buf = scene.render(&Camera::default(), 120, 90, true);
        assert_eq!(buf.len(), 120 * 90 * 4);
        assert!(buf.chunks(4).any(|p| p[3] == 255));
        assert!(buf.chunks(4).any(|p| p[3] == 0));

        let mut legacy = skin.clone();
        for y in 32..TEX {
            for x in 0..TEX {
                legacy.set(x, y, [0; 4]);
            }
        }
        legacy.upgrade_legacy();
        assert_eq!(legacy.get(24, 56), skin.get(24, 56)); // left leg front
        assert_eq!(legacy.get(38, 56), skin.get(38, 56)); // left arm front
    }
}
