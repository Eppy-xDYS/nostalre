//! Which faces of each lamp model glow: their glass, flame or bulb, picked by hand from the lamp's textures (S2d,
//! D43). A face glows when its UV centre falls in one of its model's rects on that texture. Models are keyed by
//! their path under `data\model\`, textures by their path under `data\texture\`, both lowercase. A lamp not
//! listed here keeps the name and mask rule (`glow::emissive`); a lamp listed with no glass (an empty iron
//! lantern, a brazier with no flame drawn) does not glow.

/// A glowing part of a lamp: on `texture`, the rect `uv` = (u0, v0, u1, v1).
pub struct Glass {
    pub texture: &'static str,
    pub uv: [f32; 4],
}

const fn glass(texture: &'static str, uv: [f32; 4]) -> Glass {
    Glass { texture, uv }
}

pub static LAMP_GLASS: &[(&str, &[Glass])] = &[
    ("bifrost\\벽꽃등01.rsm", &[glass("bifrost\\bif_flw01.bmp", [0.00, 0.00, 0.66, 0.50])]),
    ("bifrost\\벽꽃등02.rsm", &[glass("bifrost\\bif_flw01.bmp", [0.00, 0.00, 0.66, 0.50])]),
    ("camp\\벽등01.rsm", &[glass("camp\\camp_ob10.bmp", [0.22, 0.35, 1.00, 0.75])]),
    ("camp\\천정등01.rsm", &[glass("camp\\camp_ob2.bmp", [0.00, 0.05, 0.45, 0.32])]),
    ("camp\\탁상등01.rsm", &[glass("camp\\camp_ob3.bmp", [0.33, 0.00, 1.00, 0.55])]),
    (
        "dewata\\다약화로01.rsm",
        &[
            glass("dewata\\indo_17.bmp", [0.55, 0.77, 0.86, 0.98]),
            glass("dewata\\indo_20.bmp", [0.42, 0.50, 0.69, 0.74]),
        ],
    ),
    (
        "dicastes\\가로등.rsm",
        &[
            glass("dicastes\\dic_right01.bmp", [0.15, 0.47, 0.43, 0.99]),
            glass("dicastes\\dic_right01.bmp", [0.67, 0.40, 0.87, 0.56]),
        ],
    ),
    (
        "dicastes\\라바일등불.rsm",
        &[
            glass("dicastes\\dic_o02.bmp", [0.74, 0.70, 0.91, 0.93]),
            glass("dicastes\\dic_o04.bmp", [0.00, 0.49, 0.12, 0.83]),
        ],
    ),
    (
        "dicastes\\벽등.rsm",
        &[
            glass("dicastes\\dic_g05.bmp", [0.22, 0.45, 0.60, 0.64]),
            glass("dicastes\\dic_g07.bmp", [0.37, 0.36, 0.66, 0.59]),
        ],
    ),
    (
        "dicastes\\상급등불.rsm",
        &[
            glass("dicastes\\dic_h017.bmp", [0.50, 0.55, 0.77, 0.96]),
            glass("dicastes\\dic_h017.bmp", [0.88, 0.14, 0.98, 0.39]),
            glass("dicastes\\dic_o02.bmp", [0.74, 0.70, 0.91, 0.93]),
        ],
    ),
    (
        "dicastes\\샹들리에.rsm",
        &[
            glass("dicastes\\dic_in_board.bmp", [0.60, 0.00, 0.92, 0.46]),
            glass("dicastes\\dic_in_board.bmp", [0.29, 0.60, 0.63, 0.86]),
        ],
    ),
    (
        "dicastes\\주점등불.rsm",
        &[
            glass("dicastes\\dic_o03.bmp", [0.24, 0.39, 0.32, 0.49]),
            glass("dicastes\\dic_o03.bmp", [0.42, 0.85, 0.99, 0.96]),
            glass("dicastes\\dic_o04.bmp", [0.00, 0.49, 0.12, 0.83]),
        ],
    ),
    ("eclage\\나무가로등01.rsm", &[glass("eclage\\ecl_leaf01.bmp", [0.72, 0.11, 1.00, 0.23])]),
    (
        "eclage\\왕궁가로등01.rsm",
        &[
            glass("eclage\\ecl_obj003.bmp", [0.65, 0.72, 1.00, 0.88]),
            glass("eclage\\ecl_obj09.bmp", [0.84, 0.53, 1.00, 0.70]),
            glass("eclage\\ecl_obj10.bmp", [0.76, 0.00, 0.88, 0.56]),
            glass("eclage\\ecl_obj10.bmp", [0.76, 0.55, 1.00, 0.74]),
            glass("eclage\\ecl_obj10.bmp", [0.87, 0.29, 1.00, 0.49]),
        ],
    ),
    (
        "eclage\\왕궁벽등.rsm",
        &[
            glass("eclage\\ecl_obj10.bmp", [0.76, 0.00, 0.88, 0.56]),
            glass("eclage\\ecl_obj10.bmp", [0.76, 0.55, 1.00, 0.74]),
            glass("eclage\\ecl_obj10.bmp", [0.87, 0.29, 1.00, 0.49]),
            glass("eclage\\ecl_obj19.bmp", [0.32, 0.00, 0.66, 0.38]),
        ],
    ),
    ("eclage\\천정조명.rsm", &[glass("eclage\\ecl_obj003.bmp", [0.65, 0.72, 1.00, 0.88])]),
    (
        "event\\event_k_ojingtorch.rsm",
        &[
            glass("event\\summer_k_02.bmp", [0.40, 0.22, 0.81, 0.47]),
            glass("event\\summer_k_02.bmp", [0.15, 0.82, 0.60, 0.98]),
            glass("event\\summer_k_03.bmp", [0.35, 0.00, 0.66, 0.14]),
        ],
    ),
    ("event\\stringlights_d_01.rsm2", &[glass("event\\event_d_01.tga", [0.00, 0.00, 0.82, 0.79])]),
    ("event\\stringlights_d_02.rsm2", &[glass("event\\event_d_01.tga", [0.00, 0.00, 0.82, 0.79])]),
    ("gevent\\lamp_e_01.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("gevent\\lamp_e_02.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("gld2\\프론_가로등.rsm", &[glass("gld2\\gld2_prt_11.bmp", [0.08, 0.73, 0.39, 0.83])]),
    ("gld2\\프론_휘장가로등.rsm", &[glass("gld2\\gld2_prt_04.bmp", [0.79, 0.00, 0.93, 0.33])]),
    ("graywolf\\brazier_e_01.rsm2", &[]),
    ("graywolf\\campfire_j_01.rsm2", &[]),
    (
        "graywolf\\candlestick_s_01.rsm2",
        &[
            glass("graywolf\\gray_s_18.bmp", [0.06, 0.60, 0.28, 0.87]),
            glass("graywolf\\gray_s_18.bmp", [0.00, 0.28, 0.09, 0.37]),
            glass("job4for\\job4f_s_22.bmp", [0.62, 0.80, 0.74, 0.88]),
        ],
    ),
    (
        "graywolf\\chandelier_s_01.rsm2",
        &[
            glass("graywolf\\gray_s_18.bmp", [0.06, 0.60, 0.28, 0.87]),
            glass("graywolf\\gray_s_18.bmp", [0.00, 0.28, 0.09, 0.37]),
            glass("graywolf\\gray_s_21.tga", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    (
        "graywolf\\chandelier_s_02.rsm2",
        &[
            glass("graywolf\\gray_s_18.bmp", [0.06, 0.60, 0.28, 0.87]),
            glass("graywolf\\gray_s_18.bmp", [0.00, 0.28, 0.09, 0.37]),
            glass("job4for\\job4f_s_22.bmp", [0.62, 0.80, 0.74, 0.88]),
        ],
    ),
    ("graywolf\\light_s_01.rsm2", &[glass("graywolf\\gray_s_28.tga", [0.00, 0.05, 1.00, 1.00])]),
    ("graywolf\\light_s_02.rsm2", &[glass("graywolf\\gray_s_28.tga", [0.00, 0.05, 1.00, 1.00])]),
    ("graywolf\\light_s_03.rsm2", &[glass("graywolf\\gray_s_28.tga", [0.00, 0.05, 1.00, 1.00])]),
    ("graywolf\\light_s_04.rsm2", &[glass("graywolf\\gray_s_28.tga", [0.00, 0.05, 1.00, 1.00])]),
    ("graywolf\\lightline_s_01.rsm2", &[glass("graywolf\\gray_s_28.tga", [0.00, 0.05, 1.00, 1.00])]),
    ("graywolf\\torch_e_01.rsm2", &[]),
    (
        "graywolf\\wallcandle_s_01.rsm2",
        &[
            glass("graywolf\\gray_s_18.bmp", [0.06, 0.60, 0.28, 0.87]),
            glass("graywolf\\gray_s_18.bmp", [0.00, 0.28, 0.09, 0.37]),
            glass("job4for\\job4f_s_22.bmp", [0.62, 0.80, 0.74, 0.88]),
        ],
    ),
    (
        "herosria\\lantern_s_01.rsm2",
        &[
            glass("herosria\\herosria_s_18.bmp", [0.18, 0.12, 0.39, 0.63]),
            glass("herosria\\herosria_s_18.bmp", [0.36, 0.00, 0.92, 0.19]),
            glass("herosria\\herosria_s_18.bmp", [0.62, 0.21, 0.73, 0.36]),
        ],
    ),
    (
        "herosria\\light_s_01.rsm2",
        &[
            glass("herosria\\herosria_s_18.bmp", [0.18, 0.12, 0.39, 0.63]),
            glass("herosria\\herosria_s_18.bmp", [0.36, 0.00, 0.92, 0.19]),
            glass("herosria\\herosria_s_18.bmp", [0.62, 0.21, 0.73, 0.36]),
        ],
    ),
    (
        "herosria\\light_s_02.rsm2",
        &[
            glass("herosria\\herosria_s_18.bmp", [0.18, 0.12, 0.39, 0.63]),
            glass("herosria\\herosria_s_18.bmp", [0.36, 0.00, 0.92, 0.19]),
            glass("herosria\\herosria_s_18.bmp", [0.62, 0.21, 0.73, 0.36]),
        ],
    ),
    (
        "herosria\\light_s_03.rsm2",
        &[
            glass("herosria\\herosria_s_18.bmp", [0.18, 0.12, 0.39, 0.63]),
            glass("herosria\\herosria_s_18.bmp", [0.36, 0.00, 0.92, 0.19]),
            glass("herosria\\herosria_s_18.bmp", [0.62, 0.21, 0.73, 0.36]),
        ],
    ),
    (
        "herosria\\streetlamp_d_01.rsm2",
        &[
            glass("herosria\\herosria_d_16.bmp", [0.67, 0.69, 0.77, 0.87]),
            glass("herosria\\herosria_d_16.bmp", [0.86, 0.84, 0.98, 0.94]),
        ],
    ),
    ("hyper\\brazier_e_01.rsm2", &[]),
    ("hyper\\lamp_e_01.rsm2", &[]),
    ("ilusion\\botlight_j_01.rsm", &[glass("ilusion\\ilus_j_109.bmp", [0.34, 0.07, 0.59, 0.19])]),
    ("ilusion\\candle_d_01.rsm2", &[glass("ilusion\\ilus_d_06.bmp", [0.52, 0.69, 0.60, 0.77])]),
    (
        "ilusion\\candlestick_j_01.rsm",
        &[
            glass("ilusion\\ilus_j_125.bmp", [0.71, 0.53, 0.91, 0.63]),
            glass("ilusion\\ilus_j_133.tga", [0.00, 0.00, 0.22, 0.40]),
            glass("ilusion\\ilus_j_133.tga", [0.96, 0.41, 1.00, 0.57]),
        ],
    ),
    (
        "ilusion\\candlestick_j_02.rsm",
        &[
            glass("ilusion\\ilus_j_125.bmp", [0.71, 0.53, 0.91, 0.63]),
            glass("ilusion\\ilus_j_133.tga", [0.00, 0.00, 0.22, 0.40]),
            glass("ilusion\\ilus_j_133.tga", [0.96, 0.41, 1.00, 0.57]),
        ],
    ),
    ("ilusion\\celight_s_01.rsm", &[glass("ilusion\\ilus_s_38.bmp", [0.29, 0.69, 0.48, 0.80])]),
    (
        "ilusion\\chandelier_j_01.rsm",
        &[
            glass("prontera_re\\prt_j_20.bmp", [0.00, 0.45, 0.27, 0.99]),
            glass("prontera_re\\prt_j_23.bmp", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    (
        "ilusion\\chandelier_j_02.rsm",
        &[
            glass("ilusion\\ilus_j_125.bmp", [0.71, 0.53, 0.91, 0.63]),
            glass("ilusion\\ilus_j_133.tga", [0.00, 0.00, 0.22, 0.40]),
            glass("ilusion\\ilus_j_133.tga", [0.96, 0.41, 1.00, 0.57]),
        ],
    ),
    (
        "ilusion\\chandelier_y_01.rsm",
        &[
            glass("ilusion\\ilus_y_03.tga", [0.00, 0.00, 1.00, 1.00]),
            glass("ilusion\\ilus_y_15.bmp", [0.65, 0.58, 0.95, 0.98]),
            glass("ilusion\\ilus_y_17.bmp", [0.37, 0.27, 0.47, 0.35]),
        ],
    ),
    ("ilusion\\fireplace_s_01.rsm", &[]),
    (
        "ilusion\\lamp_y_01.rsm",
        &[
            glass("ilusion\\ilus_y_18.bmp", [0.14, 0.18, 0.28, 0.32]),
            glass("ilusion\\ilus_y_20.tga", [0.00, 0.00, 1.00, 0.76]),
            glass("ilusion\\ilus_y_21.bmp", [0.56, 0.29, 0.74, 0.39]),
            glass("ilusion\\ilus_y_21.bmp", [0.14, 0.30, 0.22, 0.38]),
        ],
    ),
    (
        "ilusion\\light_j_01.rsm",
        &[
            glass("ilusion\\ilus_j_11.bmp", [0.74, 0.66, 0.88, 0.91]),
            glass("ilusion\\ilus_j_80.bmp", [0.37, 0.11, 0.46, 0.21]),
        ],
    ),
    ("ilusion\\light_j_02.rsm", &[glass("ilusion\\ilus_j_11.bmp", [0.74, 0.66, 0.88, 0.91])]),
    ("ilusion\\light_j_03.rsm2", &[glass("ia\\ia_j_02.bmp", [0.63, 0.25, 0.91, 0.93])]),
    ("ilusion\\light_j_04.rsm2", &[glass("ilusion\\ilus_j_164.bmp", [0.00, 0.00, 1.00, 0.80])]),
    (
        "ilusion\\light_j_05.rsm2",
        &[
            glass("ilusion\\ilus_j_154.bmp", [0.30, 0.85, 0.45, 1.00]),
            glass("ilusion\\ilus_j_163.tga", [0.00, 0.00, 0.16, 0.18]),
        ],
    ),
    (
        "ilusion\\light_j_06.rsm2",
        &[
            glass("ilusion\\ilus_j_154.bmp", [0.30, 0.85, 0.45, 1.00]),
            glass("ilusion\\ilus_j_163.tga", [0.00, 0.00, 0.16, 0.18]),
        ],
    ),
    (
        "ilusion\\light_j_07.rsm2",
        &[
            glass("ilusion\\ilus_j_154.bmp", [0.30, 0.85, 0.45, 1.00]),
            glass("ilusion\\ilus_j_163.tga", [0.00, 0.00, 0.16, 0.18]),
        ],
    ),
    (
        "ilusion\\light_y_01.rsm",
        &[
            glass("ilusion\\ilus_y_03.tga", [0.00, 0.00, 1.00, 1.00]),
            glass("ilusion\\ilus_y_15.bmp", [0.65, 0.58, 0.95, 0.98]),
            glass("ilusion\\ilus_y_17.bmp", [0.37, 0.27, 0.47, 0.35]),
        ],
    ),
    (
        "ilusion\\light_y_02.rsm",
        &[
            glass("ilusion\\ilus_y_15.bmp", [0.65, 0.58, 0.95, 0.98]),
            glass("ilusion\\ilus_y_18.bmp", [0.14, 0.18, 0.28, 0.32]),
        ],
    ),
    ("ilusion\\lightin_j_01.rsm2", &[glass("ilusion\\ilus_j_71.bmp", [0.10, 0.44, 0.31, 0.69])]),
    ("ilusion\\streetlamp_y_01.rsm", &[glass("ilusion\\ilus_y_12.bmp", [0.65, 0.70, 0.74, 0.87])]),
    ("ilusion\\walllight_j_01.rsm", &[glass("ilusion\\ilus_j_111.bmp", [0.00, 0.58, 0.42, 0.79])]),
    (
        "ilusion\\walllight_s_01.rsm",
        &[
            glass("ilusion\\ilus_s_13.bmp", [0.00, 0.55, 0.25, 0.63]),
            glass("ilusion\\ilus_s_37.tga", [0.00, 0.00, 0.66, 1.00]),
        ],
    ),
    (
        "ilusion\\walllight_s_02.rsm",
        &[
            glass("ilusion\\ilus_s_72.bmp", [0.11, 0.76, 0.26, 0.96]),
            glass("ilusion\\ilus_y_03.tga", [0.00, 0.00, 1.00, 1.00]),
            glass("ilusion\\ilus_y_17.bmp", [0.37, 0.27, 0.47, 0.35]),
        ],
    ),
    (
        "ilusion\\xlight_j_01.rsm",
        &[
            glass("ilusion\\ilus_j_11.bmp", [0.74, 0.66, 0.88, 0.91]),
            glass("ilusion\\ilus_j_80_c.bmp", [0.37, 0.11, 0.46, 0.21]),
        ],
    ),
    (
        "ilusion\\xlight_j_01_01.rsm",
        &[
            glass("ilusion\\ilus_j_11.bmp", [0.74, 0.66, 0.88, 0.91]),
            glass("ilusion\\ilus_j_80_c.bmp", [0.37, 0.11, 0.46, 0.21]),
        ],
    ),
    (
        "issgard\\chandelier_d_01.rsm2",
        &[
            glass("issgard\\issgard_d_02.bmp", [0.52, 0.17, 0.63, 0.30]),
            glass("issgard\\issgard_d_02.bmp", [0.30, 0.57, 0.41, 0.68]),
            glass("issgard\\issgard_d_02.bmp", [0.08, 0.35, 0.16, 0.49]),
            glass("issgard\\issgard_d_02.bmp", [0.08, 0.24, 0.25, 0.30]),
        ],
    ),
    (
        "issgard\\chandelier_h_01.rsm2",
        &[
            glass("ilusion\\ilus_y_03.tga", [0.00, 0.00, 1.00, 1.00]),
            glass("issgard\\issgard_h_02.bmp", [0.69, 0.57, 0.93, 0.98]),
            glass("issgard\\issgard_h_03.bmp", [0.37, 0.27, 0.47, 0.35]),
        ],
    ),
    (
        "issgard\\light_s_01.rsm2",
        &[
            glass("issgard\\issgard_s_28.bmp", [0.00, 0.25, 0.11, 0.46]),
            glass("issgard\\issgard_s_28.bmp", [0.32, 0.42, 0.42, 0.52]),
        ],
    ),
    (
        "issgard\\light_s_02.rsm2",
        &[
            glass("issgard\\issgard_s_28.bmp", [0.00, 0.25, 0.11, 0.46]),
            glass("issgard\\issgard_s_28.bmp", [0.32, 0.42, 0.42, 0.52]),
            glass("issgard\\issgard_s_39.bmp", [0.05, 0.05, 0.95, 0.95]),
        ],
    ),
    (
        "issgard\\streetlamp_s_01.rsm2",
        &[
            glass("issgard\\issgard_s_28.bmp", [0.00, 0.25, 0.11, 0.46]),
            glass("issgard\\issgard_s_28.bmp", [0.32, 0.42, 0.42, 0.52]),
        ],
    ),
    ("job4\\fireplace_d_01.rsm2", &[]),
    (
        "job4for\\chandelier_s_01.rsm2",
        &[
            glass("job4for\\job4f_s_07.bmp", [0.40, 0.30, 0.48, 0.39]),
            glass("job4for\\job4f_s_07.bmp", [0.86, 0.58, 0.94, 0.69]),
            glass("job4for\\job4f_s_08.tga", [0.00, 0.00, 0.62, 0.32]),
            glass("job4for\\job4f_s_22.bmp", [0.62, 0.80, 0.74, 0.88]),
        ],
    ),
    (
        "job4for\\chandelier_s_02.rsm2",
        &[
            glass("job4for\\job4f_s_07.bmp", [0.40, 0.30, 0.48, 0.39]),
            glass("job4for\\job4f_s_07.bmp", [0.86, 0.58, 0.94, 0.69]),
            glass("job4for\\job4f_s_22.bmp", [0.62, 0.80, 0.74, 0.88]),
        ],
    ),
    ("job4for\\fireplace_s_01.rsm2", &[]),
    ("job4for\\lamp_e_01.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("job4for\\lamp_e_02.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("job4for\\lamp_e_03.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("job4for\\lamp_e_04.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    (
        "job4for\\light_s_01.rsm2",
        &[
            glass("ilusion\\ilus_y_03.tga", [0.00, 0.00, 1.00, 1.00]),
            glass("job4for\\job4f_s_07.bmp", [0.40, 0.30, 0.48, 0.39]),
            glass("job4for\\job4f_s_07.bmp", [0.86, 0.58, 0.94, 0.69]),
            glass("job4for\\job4f_s_12.bmp", [0.00, 0.62, 0.12, 0.83]),
        ],
    ),
    ("job4for\\pillerlamp_e_01.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("job4for\\pillerlamp_e_02.rsm2", &[glass("job4for\\job4for_e_03.bmp", [0.72, 0.61, 0.86, 0.71])]),
    ("job4for\\pillerlamp_e_03.rsm2", &[glass("job4for\\job4for_e_04.tga", [0.60, 0.00, 1.00, 0.50])]),
    (
        "job4for\\ylight_s_01.rsm2",
        &[
            glass("job4for\\job4f_s_07.bmp", [0.40, 0.30, 0.48, 0.39]),
            glass("job4for\\job4f_s_07.bmp", [0.86, 0.58, 0.94, 0.69]),
        ],
    ),
    ("lasagna\\lamp_01.rsm", &[glass("lasagna\\lasagna_j_06.bmp", [0.00, 0.00, 1.00, 1.00])]),
    ("lasagna\\lamp_02.rsm", &[glass("lasagna\\lasagna_j_06.bmp", [0.00, 0.00, 1.00, 1.00])]),
    (
        "lasagna\\streetlamp_s_01.rsm",
        &[
            glass("lasagna\\lasagna_s_13.bmp", [0.86, 0.74, 0.93, 0.96]),
            glass("lasagna\\lasagna_s_13.bmp", [0.59, 0.61, 0.67, 0.68]),
            glass("lasagna\\lasagna_s_13.bmp", [0.55, 0.77, 0.62, 0.83]),
        ],
    ),
    (
        "lasagna\\streetlamp_s_02.rsm",
        &[
            glass("lasagna\\lasagna_s_13.bmp", [0.86, 0.74, 0.93, 0.96]),
            glass("lasagna\\lasagna_s_13.bmp", [0.59, 0.61, 0.67, 0.68]),
            glass("lasagna\\lasagna_s_13.bmp", [0.55, 0.77, 0.62, 0.83]),
        ],
    ),
    ("lifemd\\torch_d_02.rsm2", &[]),
    (
        "malangdo\\벽등.rsm",
        &[
            glass("malangdo\\mal_15.bmp", [0.59, 0.36, 0.75, 0.80]),
            glass("malangdo\\mal_15.bmp", [0.32, 0.29, 0.50, 0.38]),
        ],
    ),
    (
        "malangdo\\천정등.rsm",
        &[
            glass("malangdo\\mal_02.bmp", [0.64, 0.29, 0.71, 0.37]),
            glass("malangdo\\mal_04.bmp", [0.31, 0.55, 0.55, 0.79]),
            glass("malangdo\\mal_15.bmp", [0.59, 0.36, 0.75, 0.80]),
            glass("malangdo\\mal_15.bmp", [0.32, 0.29, 0.50, 0.38]),
            glass("malangdo\\mal_18.bmp", [0.00, 0.28, 0.39, 0.48]),
        ],
    ),
    (
        "malaya\\가로등.rsm",
        &[
            glass("malaya\\ma_field_flame01.bmp", [0.76, 0.21, 0.93, 0.41]),
            glass("malaya\\ma_field_flame01.bmp", [0.71, 0.52, 0.81, 0.66]),
        ],
    ),
    ("malaya\\내부벽등01.rsm", &[glass("malaya\\ma_table2.bmp", [0.77, 0.64, 0.99, 0.93])]),
    ("malaya\\내부샹들리에01.rsm", &[glass("malaya\\ma_handrail.tga", [0.28, 0.07, 0.69, 0.43])]),
    ("malaya\\병원조명01.rsm", &[glass("malaya\\ma_field_plants18.bmp", [0.85, 0.70, 1.00, 0.77])]),
    ("malaya\\수술실조명01.rsm", &[]),
    ("malaya\\화산화로.rsm", &[glass("malaya\\ma_lake_obj01.bmp", [0.50, 0.64, 0.87, 0.96])]),
    (
        "manuk\\관리소비상등01.rsm",
        &[
            glass("manuk\\manuk_in3.bmp", [0.59, 0.78, 0.85, 0.90]),
            glass("manuk\\manuk_in5.bmp", [0.78, 0.33, 0.89, 0.50]),
        ],
    ),
    (
        "manuk\\관리소횃불01.rsm",
        &[
            glass("manuk\\manuk_in3.bmp", [0.59, 0.78, 0.85, 0.90]),
            glass("manuk\\manuk_in5.bmp", [0.78, 0.33, 0.89, 0.50]),
        ],
    ),
    (
        "manuk\\민가바닥등.rsm",
        &[
            glass("manuk\\manuk_in1.bmp", [0.74, 0.00, 0.93, 0.56]),
            glass("manuk\\manuk_in1.bmp", [0.67, 0.54, 0.87, 0.76]),
            glass("manuk\\manuk_in1.tga", [0.74, 0.00, 0.93, 0.58]),
            glass("manuk\\manuk_in1.tga", [0.60, 0.56, 0.82, 0.87]),
        ],
    ),
    ("neomd\\chandelier_j_01.rsm2", &[glass("neomd\\neomd_j_05.tga", [0.00, 0.00, 0.15, 0.16])]),
    ("oldcastle\\chandelier01.rsm", &[glass("oldcastle\\oc_006.bmp", [0.50, 0.00, 0.82, 0.34])]),
    ("oldcastle\\chandelier02.rsm", &[glass("oldcastle\\oc_006.bmp", [0.50, 0.00, 0.82, 0.34])]),
    ("oldcastle\\fireplace.rsm", &[]),
    ("para\\candle.rsm", &[glass("masin\\masin_002.bmp", [0.16, 0.55, 0.26, 0.63])]),
    (
        "para\\para_k_light.rsm",
        &[
            glass("para\\para_k_2.bmp", [0.00, 0.00, 0.42, 0.39]),
            glass("para\\para_k_2.bmp", [0.64, 0.00, 1.00, 0.42]),
        ],
    ),
    (
        "para\\para_k_twolight.rsm",
        &[
            glass("para\\para_k_2.bmp", [0.00, 0.00, 0.42, 0.39]),
            glass("para\\para_k_2.bmp", [0.64, 0.00, 1.00, 0.42]),
            glass("para\\para_k_4.bmp", [0.59, 0.17, 1.00, 1.00]),
        ],
    ),
    ("pop\\lamp_h_01.rsm", &[glass("pop\\pop_h_01.bmp", [0.29, 0.29, 0.40, 0.44])]),
    ("pop\\slamp_h_01.rsm", &[glass("pop\\pop_h_01.bmp", [0.29, 0.29, 0.40, 0.44])]),
    (
        "prontera\\chandelier.rsm",
        &[
            glass("prontera\\prt_j_03.bmp", [0.74, 0.49, 0.86, 0.58]),
            glass("prontera\\prt_j_03.bmp", [0.87, 0.60, 0.92, 0.72]),
            glass("prontera\\prt_j_05.bmp", [0.87, 0.00, 1.00, 0.33]),
        ],
    ),
    ("prontera\\fire_thing.rsm", &[glass("prontera\\prt_j_06.bmp", [0.07, 0.27, 0.16, 0.37])]),
    (
        "prontera\\fireplace.rsm",
        &[
            glass("prontera\\prt_j_05.bmp", [0.87, 0.00, 1.00, 0.33]),
            glass("prontera\\prt_j_07.bmp", [0.10, 0.00, 0.65, 0.35]),
        ],
    ),
    (
        "prontera\\light.rsm",
        &[
            glass("prontera\\prt_j_03.bmp", [0.74, 0.49, 0.86, 0.58]),
            glass("prontera\\prt_j_03.bmp", [0.87, 0.60, 0.92, 0.72]),
            glass("prontera\\prt_j_05.bmp", [0.87, 0.00, 1.00, 0.33]),
            glass("prontera\\prt_j_06.bmp", [0.07, 0.27, 0.16, 0.37]),
        ],
    ),
    (
        "prontera\\prt_k_light_1.rsm",
        &[
            glass("prontera\\prt_k_01.bmp", [0.42, 0.19, 0.54, 0.29]),
            glass("prontera\\prt_k_01.bmp", [0.32, 0.32, 0.48, 0.53]),
            glass("prontera\\prt_k_01.bmp", [0.30, 0.63, 0.49, 0.76]),
        ],
    ),
    (
        "prontera\\prt_k_light_2.rsm",
        &[
            glass("prontera\\prt_k_02.bmp", [0.07, 0.54, 0.19, 0.67]),
            glass("prontera\\prt_k_06.bmp", [0.00, 0.86, 0.06, 0.99]),
        ],
    ),
    (
        "prontera\\prt_k_light_3.rsm",
        &[
            glass("prontera\\prt_k_04.bmp", [0.05, 0.08, 0.26, 0.23]),
            glass("prontera\\prt_k_06.bmp", [0.00, 0.86, 0.06, 0.99]),
        ],
    ),
    (
        "prontera\\prt_k_ojingtorch.rsm",
        &[
            glass("event\\summer_k_02.bmp", [0.40, 0.22, 0.81, 0.47]),
            glass("event\\summer_k_02.bmp", [0.15, 0.82, 0.60, 0.98]),
            glass("event\\summer_k_03.bmp", [0.35, 0.00, 0.66, 0.14]),
        ],
    ),
    ("prontera\\torch_01.rsm", &[]),
    ("prontera_re\\candle_j_01.rsm", &[glass("prontera_re\\prt_j_18.bmp", [0.91, 0.00, 1.00, 0.21])]),
    (
        "prontera_re\\chandelier_j.rsm",
        &[
            glass("prontera_re\\prt_j_20.bmp", [0.00, 0.45, 0.27, 0.99]),
            glass("prontera_re\\prt_j_23.bmp", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    ("rockridge\\lamp_s_01.rsm", &[glass("rockridge\\ridge_s_07.bmp", [0.34, 0.06, 0.50, 0.14])]),
    ("rockridge\\lit_j_01.rsm", &[glass("rockridge\\ridge_j_17.bmp", [0.66, 0.00, 0.79, 0.13])]),
    ("rockridge\\lit_j_02.rsm", &[glass("rockridge\\ridge_j_17.bmp", [0.66, 0.00, 0.79, 0.13])]),
    ("rockridge\\streetlamp_s_01.rsm", &[glass("rockridge\\ridge_s_19.tga", [0.00, 0.00, 1.00, 1.00])]),
    ("rockridge\\streetlamp_s_02.rsm", &[glass("rockridge\\ridge_s_19.tga", [0.00, 0.00, 1.00, 1.00])]),
    ("splen\\가로등01.rsm", &[glass("splen\\splen_eo1.bmp", [0.43, 0.05, 0.95, 1.00])]),
    (
        "splen\\가로등02.rsm",
        &[
            glass("splen\\splen_eo1.bmp", [0.43, 0.05, 0.95, 1.00]),
            glass("splen\\splen_o15.bmp", [0.47, 0.50, 1.00, 1.00]),
        ],
    ),
    ("splen\\민가등01.rsm", &[glass("splen\\splen_in_prison01.bmp", [0.65, 0.71, 0.99, 1.00])]),
    (
        "splen\\주점등01.rsm",
        &[
            glass("splen\\splen_in9.bmp", [0.14, 0.29, 0.28, 0.44]),
            glass("splen\\splen_in9.bmp", [0.12, 0.54, 0.28, 0.66]),
            glass("splen\\splen_in9.bmp", [0.47, 0.47, 0.63, 0.64]),
        ],
    ),
    (
        "splen\\주점등02.rsm",
        &[
            glass("splen\\splen_in10.bmp", [0.36, 0.32, 0.43, 0.43]),
            glass("splen\\splen_in10.bmp", [0.68, 0.00, 0.86, 0.76]),
        ],
    ),
    (
        "splen\\주점벽등01.rsm",
        &[
            glass("splen\\splen_in9.bmp", [0.14, 0.29, 0.28, 0.44]),
            glass("splen\\splen_in9.bmp", [0.12, 0.54, 0.28, 0.66]),
            glass("splen\\splen_in9.bmp", [0.47, 0.47, 0.63, 0.64]),
        ],
    ),
    (
        "splen\\주점벽등02.rsm",
        &[
            glass("splen\\splen_in10.bmp", [0.36, 0.32, 0.43, 0.43]),
            glass("splen\\splen_in10.bmp", [0.68, 0.00, 0.86, 0.76]),
        ],
    ),
    (
        "tera\\desklight_s_01.rsm",
        &[
            glass("tera\\tera_s_13.bmp", [0.18, 0.70, 0.46, 0.98]),
            glass("tera\\tera_s_13.bmp", [0.50, 0.27, 0.61, 0.43]),
        ],
    ),
    (
        "tera\\lamp_s_01.rsm",
        &[
            glass("tera\\tera_h_02.bmp", [0.00, 0.40, 0.09, 0.71]),
            glass("tera\\tera_h_02.bmp", [0.24, 0.21, 0.34, 0.48]),
        ],
    ),
    (
        "tera\\lamp_s_02.rsm",
        &[
            glass("tera\\tera_h_02.bmp", [0.00, 0.40, 0.09, 0.71]),
            glass("tera\\tera_h_02.bmp", [0.24, 0.21, 0.34, 0.48]),
        ],
    ),
    (
        "tera\\light_s_01.rsm",
        &[
            glass("tera\\tera_s_13.bmp", [0.18, 0.70, 0.46, 0.98]),
            glass("tera\\tera_s_13.bmp", [0.50, 0.27, 0.61, 0.43]),
        ],
    ),
    (
        "tera\\light_s_02.rsm",
        &[
            glass("tera\\tera_s_13.bmp", [0.18, 0.70, 0.46, 0.98]),
            glass("tera\\tera_s_13.bmp", [0.50, 0.27, 0.61, 0.43]),
        ],
    ),
    (
        "tera\\light_s_03.rsm",
        &[
            glass("tera\\tera_s_13.bmp", [0.18, 0.70, 0.46, 0.98]),
            glass("tera\\tera_s_13.bmp", [0.50, 0.27, 0.61, 0.43]),
        ],
    ),
    (
        "tera\\light_s_04.rsm",
        &[
            glass("tera\\tera_s_13.bmp", [0.18, 0.70, 0.46, 0.98]),
            glass("tera\\tera_s_13.bmp", [0.50, 0.27, 0.61, 0.43]),
        ],
    ),
    ("undying\\chandelier_y_01.rsm2", &[glass("undying\\undying_y_07.tga", [0.00, 0.00, 1.00, 1.00])]),
    ("undying\\chandelier_y_02.rsm2", &[glass("undying\\undying_y_07.tga", [0.00, 0.00, 1.00, 1.00])]),
    ("undying\\chandelier_y_03.rsm2", &[glass("undying\\undying_y_07.tga", [0.00, 0.00, 1.00, 1.00])]),
    (
        "verus\\h_lamp.rsm",
        &[
            glass("verus\\ver_h_07.bmp", [0.64, 0.12, 0.73, 0.33]),
            glass("verus\\ver_h_07.bmp", [0.36, 0.23, 0.61, 0.33]),
        ],
    ),
    (
        "verus\\lamp_01.rsm",
        &[
            glass("verus\\ver_h_25.bmp", [0.81, 0.21, 0.91, 0.54]),
            glass("verus\\ver_h_25.bmp", [0.81, 0.67, 0.91, 0.99]),
        ],
    ),
    (
        "verus\\lamp_02.rsm",
        &[
            glass("verus\\ver_h_25.bmp", [0.81, 0.21, 0.91, 0.54]),
            glass("verus\\ver_h_25.bmp", [0.81, 0.67, 0.91, 0.99]),
        ],
    ),
    (
        "verus\\lighting_01.rsm",
        &[
            glass("verus\\ver_h_35.bmp", [0.60, 0.86, 0.76, 0.93]),
            glass("verus\\ver_h_35.bmp", [0.04, 0.84, 0.26, 0.91]),
        ],
    ),
    (
        "verus\\robot_light1.rsm",
        &[
            glass("verus\\ver_k_18_1.bmp", [0.47, 0.29, 0.61, 0.43]),
            glass("verus\\ver_k_18_1.bmp", [0.17, 0.71, 0.38, 0.80]),
        ],
    ),
    (
        "verus\\robot_light2.rsm",
        &[
            glass("verus\\ver_k_18_1.bmp", [0.47, 0.29, 0.61, 0.43]),
            glass("verus\\ver_k_18_1.bmp", [0.17, 0.71, 0.38, 0.80]),
        ],
    ),
    (
        "verus\\robot_light3.rsm",
        &[
            glass("verus\\ver_k_18_1.bmp", [0.47, 0.29, 0.61, 0.43]),
            glass("verus\\ver_k_18_1.bmp", [0.17, 0.71, 0.38, 0.80]),
        ],
    ),
    ("거북이섬\\거북_외부등.rsm", &[glass("내부소품\\turtle_isl02.bmp", [0.08, 0.38, 0.54, 0.98])]),
    ("글래스트\\글래스트_외부등.rsm", &[glass("외부소품\\gl_ramp01.bmp", [0.00, 0.33, 0.15, 0.60])]),
    ("내부소품\\대기실횃불.rsm", &[glass("내부소품\\arena1-3.bmp", [0.83, 0.40, 1.00, 0.55])]),
    (
        "내부소품\\실내등1.rsm",
        &[
            glass("내부소품\\l-side1.bmp", [0.00, 0.00, 1.00, 1.00]),
            glass("내부소품\\l-side3.bmp", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    (
        "내부소품\\실내등2.rsm",
        &[
            glass("내부소품\\l-side1.bmp", [0.00, 0.00, 1.00, 1.00]),
            glass("내부소품\\l-side3.bmp", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    ("내부소품\\실내등3.rsm", &[glass("내부소품\\ind2-side1.bmp", [0.00, 0.00, 1.00, 1.00])]),
    (
        "내부소품\\실내등4.rsm",
        &[
            glass("내부소품\\ind2-side1.bmp", [0.00, 0.00, 1.00, 1.00]),
            glass("내부소품\\ind3-side1.bmp", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    ("내부소품\\실내등6.rsm", &[glass("내부소품\\delight1.bmp", [0.00, 0.00, 1.00, 1.00])]),
    (
        "내부소품\\실내등7.rsm",
        &[
            glass("내부소품\\sad_01.bmp", [0.85, 0.69, 1.00, 1.00]),
            glass("내부소품\\sad_02.bmp", [0.00, 0.00, 1.00, 1.00]),
            glass("내부소품\\sad_03.bmp", [0.00, 0.00, 1.00, 1.00]),
        ],
    ),
    ("내부소품\\오크화로.rsm", &[glass("내부소품\\okff-2.bmp", [0.05, 0.08, 0.95, 0.88])]),
    ("내부소품\\지하등1.rsm", &[]),
    ("내부소품\\페이욘등.rsm", &[glass("pe_r_02 copy.bmp", [0.00, 0.00, 0.38, 0.84])]),
    ("내부소품\\프론아레나1-횃불.rsm", &[glass("내부소품\\arena1-3.bmp", [0.83, 0.40, 1.00, 0.55])]),
    ("니플헤임\\니플헤임-가로등.rsm", &[glass("기타마을\\ni_r01.bmp", [0.74, 0.00, 0.99, 0.25])]),
    ("대만\\대만_촛불.rsm", &[glass("기타마을내부\\taifire_01.bmp", [0.61, 0.38, 0.85, 0.98])]),
    ("라헬\\가로등.rsm", &[glass("기타마을\\tru_jap3 copy.bmp", [0.63, 0.00, 0.98, 0.33])]),
    ("라헬\\둥근등.rsm", &[glass("기타마을\\rc_wall005.bmp", [0.76, 0.41, 0.86, 0.61])]),
    ("라헬\\샹들리에.rsm", &[glass("기타마을내부\\rachel_chandelier.bmp", [0.57, 0.20, 1.00, 1.00])]),
    ("러시아\\샹들리에.rsm", &[glass("내부소품\\russia_saen copy.bmp", [0.18, 0.00, 0.30, 0.20])]),
    ("리히타르젠\\가로등01.rsm", &[glass("기타마을\\lt_ob01.bmp", [0.02, 0.25, 0.40, 0.60])]),
    ("리히타르젠\\가로등02.rsm", &[glass("기타마을\\lt_ob01.bmp", [0.02, 0.25, 0.40, 0.60])]),
    ("리히타르젠\\가로등03.rsm", &[glass("기타마을\\lt_ob01.bmp", [0.02, 0.25, 0.40, 0.60])]),
    ("리히타르젠\\가로등04.rsm", &[glass("기타마을\\lt_ob01.bmp", [0.02, 0.25, 0.40, 0.60])]),
    ("리히타르젠\\가로등05.rsm", &[glass("기타마을\\lt_ob01.bmp", [0.02, 0.25, 0.40, 0.60])]),
    (
        "리히타르젠\\벽등01.rsm",
        &[
            glass("lighthalzen\\lhz_etc11.bmp", [0.05, 0.17, 0.26, 0.29]),
            glass("lighthalzen\\lhz_etc11.bmp", [0.53, 0.60, 0.64, 0.99]),
            glass("lighthalzen\\lhz_etc11.bmp", [0.10, 0.82, 0.25, 0.95]),
            glass("lighthalzen\\lhz_etc11.tga", [0.05, 0.17, 0.26, 0.29]),
            glass("lighthalzen\\lhz_etc11.tga", [0.53, 0.60, 0.64, 0.99]),
            glass("lighthalzen\\lhz_etc11.tga", [0.10, 0.82, 0.25, 0.95]),
        ],
    ),
    ("리히타르젠\\샹들리에.rsm", &[glass("기타마을\\busi_inri.bmp", [0.92, 0.77, 1.00, 0.95])]),
    ("리히타르젠\\신호등.rsm", &[glass("기타마을\\com_sinho copy.bmp", [0.00, 0.03, 0.54, 0.29])]),
    ("모로코\\피라밋횃불.rsm", &[]),
    ("무명섬\\가로등.rsm", &[glass("기타마을\\cursed_flag01.bmp", [0.69, 0.54, 0.90, 0.96])]),
    ("무명섬\\가로등02.rsm", &[glass("기타마을\\k_gesi2 copy.bmp", [0.74, 0.04, 0.98, 0.33])]),
    (
        "무명섬\\샹들리에.rsm",
        &[
            glass("내부소품\\cursed_item01.bmp", [0.07, 0.60, 0.14, 0.74]),
            glass("내부소품\\cursed_item01.bmp", [0.60, 0.27, 0.74, 0.67]),
        ],
    ),
    ("사막도시\\가로등.rsm", &[glass("기타마을\\desert_direction.bmp", [0.72, 0.57, 0.92, 1.00])]),
    ("아요타야\\샹들리에.rsm", &[glass("기타마을내부\\t_inob2.bmp", [0.25, 0.37, 0.70, 1.00])]),
    ("아요타야\\화로.rsm", &[]),
    ("아인브로크\\가로등.rsm", &[glass("기타마을\\st_obga.bmp", [0.00, 0.43, 0.34, 0.71])]),
    ("아인브로크\\경보등.rsm", &[glass("기타마을내부\\st_ino10.bmp", [0.88, 0.79, 1.00, 0.97])]),
    (
        "아인브로크\\등01.rsm",
        &[
            glass("기타마을내부\\st_ino002.bmp", [0.07, 0.48, 0.49, 0.89]),
            glass("기타마을내부\\st_ino002.bmp", [0.51, 0.66, 0.72, 0.87]),
        ],
    ),
    ("아인브로크\\등02.rsm", &[glass("기타마을내부\\st_ino003.bmp", [0.27, 0.57, 0.69, 0.97])]),
    ("아인브로크\\화로.rsm", &[glass("기타마을내부\\st_ino004.bmp", [0.06, 0.78, 0.40, 0.95])]),
    ("아인브로크\\화로2.rsm", &[glass("기타마을내부\\st_ino004.bmp", [0.06, 0.78, 0.40, 0.95])]),
    ("외부소품\\벽가로등.rsm", &[glass("기타마을\\alde_entrance03.bmp", [0.89, 0.06, 0.96, 0.18])]),
    ("유노추가\\유노_샹들리에.rsm", &[glass("기타마을\\uno_shag03.bmp", [0.00, 0.20, 1.00, 0.76])]),
    ("인던02\\인던02보스횃불대01.rsm", &[glass("내부소품\\dg2_bosschair_01.bmp", [0.08, 0.20, 0.16, 0.60])]),
    ("일본\\일본_등불.rsm", &[glass("기타마을\\japlofire_01.bmp", [0.13, 0.00, 0.40, 0.64])]),
    ("전장\\전장1b건물03화로.rsm", &[glass("외부소품\\bf1_stg05.bmp", [0.00, 0.38, 0.25, 0.60])]),
    ("전장\\전장2b용장식횃불대.rsm", &[]),
    ("중국\\중국-등.rsm", &[glass("기타마을\\ch_f01.bmp", [0.00, 0.00, 1.00, 1.00])]),
    (
        "중국\\중국-화로.rsm",
        &[
            glass("기타마을내부\\ch_firepot02.bmp", [0.08, 0.00, 0.48, 0.30]),
            glass("기타마을내부\\ch_firepot02.bmp", [0.48, 0.00, 0.88, 0.37]),
        ],
    ),
    ("중국\\중국-화로02.rsm", &[glass("기타마을내부\\ch_firepot.bmp", [0.59, 0.13, 0.92, 0.57])]),
    ("지하감옥\\지하감옥_화로1.rsm", &[glass("내부소품\\seokse copy.bmp", [0.10, 0.00, 0.84, 0.30])]),
    ("크리스마스마을\\xmas_가로등.rsm", &[glass("기타마을\\xmas_lt01.bmp", [0.49, 0.37, 0.63, 0.61])]),
    ("크리스마스마을\\xmas_신호등.rsm", &[glass("기타마을\\xmas_train04.bmp", [0.00, 0.70, 0.37, 0.85])]),
    ("타나토스\\샹들리에01.rsm", &[glass("내부소품\\tana_sang.bmp", [0.87, 0.07, 1.00, 0.24])]),
    ("타나토스\\샹들리에02.rsm", &[glass("내부소품\\tana_sang.bmp", [0.87, 0.07, 1.00, 0.24])]),
    ("타나토스\\샹들리에03.rsm", &[glass("내부소품\\tana_sang2 copy.bmp", [0.87, 0.07, 1.00, 0.24])]),
    ("토르화산\\전등.rsm", &[glass("내부소품\\tor_pro.bmp", [0.31, 0.60, 0.42, 0.83])]),
    ("페이욘\\등.rsm", &[glass("기타마을내부\\py_ob006.bmp", [0.47, 0.00, 0.96, 0.27])]),
    ("페이욘\\화로.rsm", &[]),
    ("프론테라\\휘장가로등.rsm", &[glass("프론테라\\d-side.bmp", [0.10, 0.03, 0.85, 0.97])]),
    ("휘겔\\휘겔_가로등01.rsm", &[glass("기타마을\\whi_garodung copy.bmp", [0.07, 0.00, 0.24, 0.26])]),
    (
        "히나마쯔리\\일본등.rsm",
        &[
            glass("외부소품\\hena.bmp", [0.27, 0.00, 1.00, 0.48]),
            glass("외부소품\\hena.bmp", [0.86, 0.50, 1.00, 1.00]),
        ],
    ),
];
