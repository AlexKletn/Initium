use lvgl_rust::sys;

#[derive(Clone, Copy)]
pub(crate) enum ChoiceLayout {
    Power,
    Userspace,
}

impl ChoiceLayout {
    pub fn tile_size(self) -> (i32, i32) {
        match self {
            Self::Power => (192, 192),
            Self::Userspace => (220, 68),
        }
    }

    pub fn label_width(self) -> i32 {
        match self {
            Self::Power => 176,
            Self::Userspace => 144,
        }
    }

    pub fn panel_padding(self) -> i32 {
        match self {
            Self::Power => 16,
            Self::Userspace => 8,
        }
    }

    pub fn column_gap(self) -> i32 {
        match self {
            Self::Power => 32,
            Self::Userspace => 8,
        }
    }

    pub fn y(self, height: i32) -> i32 {
        match self {
            Self::Power => 111 + height / 2,
            Self::Userspace => -180,
        }
    }

    pub fn flow(self) -> sys::lv_flex_flow_t {
        match self {
            Self::Power => sys::lv_flex_flow_t_LV_FLEX_FLOW_COLUMN,
            Self::Userspace => sys::lv_flex_flow_t_LV_FLEX_FLOW_ROW,
        }
    }
}
