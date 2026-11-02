//! The caustics read the waves' bend from `gnoise_laplacian` (water_waves.wgsl). A CPU copy of the noise and of
//! that formula, checked against finite differences of the value, proves the formula.

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn hash_grad(p: [f32; 2]) -> [f32; 2] {
    let mut p3 = [
        fract(p[0] * 0.1031),
        fract(p[1] * 0.1030),
        fract(p[0] * 0.0973),
    ];
    let d = p3[0] * (p3[1] + 33.33) + p3[1] * (p3[2] + 33.33) + p3[2] * (p3[0] + 33.33);
    for v in &mut p3 {
        *v += d;
    }
    [
        fract((p3[0] + p3[1]) * p3[2]) * 2.0 - 1.0,
        fract((p3[0] + p3[2]) * p3[1]) * 2.0 - 1.0,
    ]
}

struct Lattice {
    u: [f32; 2],
    du: [f32; 2],
    ddu: [f32; 2],
    g: [[f32; 2]; 4],
    v: [f32; 4],
}

fn lattice(p: [f32; 2]) -> Lattice {
    let i = [p[0].floor(), p[1].floor()];
    let f = [p[0] - i[0], p[1] - i[1]];
    let fade = |f: f32| f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let dfade = |f: f32| 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let ddfade = |f: f32| 60.0 * f * (f * (2.0 * f - 3.0) + 1.0);
    let corners = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    let g = corners.map(|c| hash_grad([i[0] + c[0], i[1] + c[1]]));
    let v =
        [0, 1, 2, 3].map(|k| g[k][0] * (f[0] - corners[k][0]) + g[k][1] * (f[1] - corners[k][1]));
    Lattice {
        u: f.map(fade),
        du: f.map(dfade),
        ddu: f.map(ddfade),
        g,
        v,
    }
}

fn value(p: [f32; 2]) -> f32 {
    let l = lattice(p);
    let [a, b, c, d] = l.v;
    (a + l.u[0] * (b - a) + l.u[1] * (c - a) + l.u[0] * l.u[1] * (a - b - c + d)) / 1.4 + 0.5
}

/// water_waves.wgsl `gnoise_laplacian`, line for line.
fn laplacian(p: [f32; 2]) -> f32 {
    let l = lattice(p);
    let [va, vb, vc, vd] = l.v;
    let [ga, gb, gc, gd] = l.g;
    let k = va - vb - vc + vd;
    let kg = [ga[0] - gb[0] - gc[0] + gd[0], ga[1] - gb[1] - gc[1] + gd[1]];
    let dxx = 2.0 * l.du[0] * (gb[0] - ga[0])
        + l.ddu[0] * (vb - va)
        + 2.0 * l.du[0] * l.u[1] * kg[0]
        + l.ddu[0] * l.u[1] * k;
    let dyy = 2.0 * l.du[1] * (gc[1] - ga[1])
        + l.ddu[1] * (vc - va)
        + 2.0 * l.du[1] * l.u[0] * kg[1]
        + l.ddu[1] * l.u[0] * k;
    (dxx + dyy) / 1.4
}

#[test]
fn laplacian_matches_finite_differences() {
    let h = 1e-2_f32;
    for i in 0..200 {
        let p = [0.137 + i as f32 * 0.731, 2.31 + i as f32 * 0.419];
        let numeric = (value([p[0] + h, p[1]])
            + value([p[0] - h, p[1]])
            + value([p[0], p[1] + h])
            + value([p[0], p[1] - h])
            - 4.0 * value(p))
            / (h * h);
        let exact = laplacian(p);
        assert!(
            (numeric - exact).abs() < 0.05 + exact.abs() * 0.02,
            "at {p:?}: {numeric} vs {exact}"
        );
    }
}

#[test]
fn shared_waves_reach_both_shaders() {
    for (name, src) in [
        ("terrain", nostalre_renderer::TERRAIN_WGSL),
        ("water_hd", nostalre_renderer::WATER_HD_WGSL),
    ] {
        assert!(src.contains("fn wave_laplacian("), "{name}");
        assert!(src.contains("fn wave3("), "{name}");
    }
}
