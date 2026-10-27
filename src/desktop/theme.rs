// SPDX-License-Identifier: Apache-2.0
//! Visual appearance lives in this single theme module, not in view controls.
//! Tokens reference the approved HTML RV Studio Render visual design.
use eframe::egui::{self,Color32};

#[derive(Clone,Copy)]
pub struct Tokens {
    pub dark:bool,
    pub accent:Color32,
    pub muted:Color32,
    pub panel:Color32,
    pub canvas:Color32,
    pub positive:Color32,
    pub left_width:f32,
    pub right_width:f32,
    pub camera_height:f32,
    pub top_height:f32,
    pub gap:f32,
    pub padding:f32,
    pub brand_size:f32,
}
impl Tokens {
    pub fn palette(dark:bool)->Self{
        let (accent,muted,panel,canvas,positive)=if dark{
            (
                Color32::from_rgb(105,168,238),
                Color32::from_rgb(164,179,192),
                Color32::from_rgb(28,39,51),
                Color32::from_rgb(16,23,35),
                Color32::from_rgb(86,199,149),
            )
        }else{
            (
                Color32::from_rgb(64,125,198),
                Color32::from_rgb(112,129,146),
                Color32::from_rgb(255,255,255),
                Color32::from_rgb(17,27,43),
                Color32::from_rgb(52,167,124),
            )
        };
        Self{
            dark,accent,muted,panel,canvas,positive,
            left_width:258.0,right_width:305.0,camera_height:170.0,
            top_height:68.0,gap:10.0,padding:16.0,brand_size:15.0
        }
    }
    pub fn apply(self,ctx:&egui::Context){
        let mut style=(*ctx.style()).clone();
        let mut visuals=if self.dark{egui::Visuals::dark()}else{egui::Visuals::light()};
        visuals.panel_fill=self.panel;
        visuals.window_fill=self.panel;
        visuals.selection.bg_fill=self.accent;
        visuals.hyperlink_color=self.accent;
        style.visuals=visuals;
        style.spacing.item_spacing=egui::vec2(self.gap,self.gap);
        ctx.set_style(style);
    }
}
