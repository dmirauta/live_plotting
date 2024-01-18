use std::error::Error;

use egui::{vec2, CentralPanel, ScrollArea, Slider, Vec2b};
use egui_extras::syntax_highlighting::{highlight, CodeTheme};
use egui_inspect::EguiInspect;
use egui_plot::{Line, Plot};
use ndarray::Array1;
use rhai::{
    packages::{
        ArithmeticPackage, BasicArrayPackage, BasicIteratorPackage, BasicMathPackage,
        LanguageCorePackage, LogicPackage, Package,
    },
    Engine, Scope, AST,
};

struct FunctionEditor {
    code: String,
    theme: CodeTheme,
}

impl Default for FunctionEditor {
    fn default() -> Self {
        Self {
            code: include_str!("polar.rh").to_string(),
            theme: Default::default(),
        }
    }
}

impl EguiInspect for FunctionEditor {
    fn inspect(&self, _label: &str, _ui: &mut egui::Ui) {
        todo!()
    }

    fn inspect_mut(&mut self, _label: &str, ui: &mut egui::Ui) {
        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = highlight(ui.ctx(), &self.theme, string, "rust");
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.code)
                        .code_editor()
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .min_size(vec2(300.0, 200.0))
                        .layouter(&mut layouter),
                );
            });

        #[cfg(not(target_arch = "wasm32"))]
        ui.horizontal(|ui| {
            ui.label("Source: ");
            if ui.button("Save").clicked() {
                if let Some(path_buf) = rfd::FileDialog::new().save_file() {
                    std::fs::write(path_buf, self.code.as_str()).unwrap();
                }
            }
            if ui.button("Load").clicked() {
                if let Some(path_buf) = rfd::FileDialog::new().pick_file() {
                    if let Ok(code) = std::fs::read_to_string(path_buf) {
                        self.code = code;
                    }
                }
            }
        });
    }
}

#[derive(Default)]
struct FuncPlotter {
    xy: Vec<[f64; 2]>,
}

impl EguiInspect for FuncPlotter {
    fn inspect(&self, label: &str, ui: &mut egui::Ui) {
        Plot::new(label)
            .view_aspect(1.0)
            .data_aspect(1.0)
            .min_size(vec2(200.0, 200.0))
            .auto_bounds(Vec2b::FALSE)
            .show(ui, |pui| pui.line(Line::new(self.xy.clone())));
    }

    fn inspect_mut(&mut self, label: &str, ui: &mut egui::Ui) {
        self.inspect(label, ui);
    }
}

struct Params {
    n_points: usize,
    t: f64,
    advance: bool,
    changed: bool,
}

static INITIAL_N_POINTS: usize = 200;

impl Default for Params {
    fn default() -> Self {
        Self {
            n_points: INITIAL_N_POINTS,
            t: Default::default(),
            advance: false,
            changed: false,
        }
    }
}

impl EguiInspect for Params {
    fn inspect(&self, _label: &str, _ui: &mut egui::Ui) {
        todo!()
    }

    fn inspect_mut(&mut self, _label: &str, ui: &mut egui::Ui) {
        ui.ctx().request_repaint();
        if self.advance {
            self.t += ui.input(|i| i.stable_dt as f64) / 4.0;
        }
        let delta = self.t - 1.0;
        if delta > 0.0 {
            self.t = delta;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("n points");
            let resp = ui.add(Slider::new(&mut self.n_points, 10..=1000).logarithmic(true));
            self.changed = self.changed || resp.changed();
            ui.label("  time");
            let resp = ui.add(
                Slider::new(&mut self.t, 0.0..=1.0)
                    .min_decimals(2)
                    .max_decimals(2),
            );
            self.changed = self.changed || resp.changed();
            ui.checkbox(&mut self.advance, "advance");
        });
    }
}

macro_rules! register_packages {
    ($engine: ident; $($t: ident),+) => {
       $($t::new().register_into_engine(&mut $engine);)*
    };
}

struct RhaiContext {
    engine: Engine,
    ast: AST,
    rhai_feedback: String,
    parsed_code: String,
}

impl Default for RhaiContext {
    fn default() -> Self {
        let mut engine = Engine::new_raw();
        register_packages!(engine; LanguageCorePackage, ArithmeticPackage, BasicIteratorPackage, LogicPackage, BasicMathPackage, BasicArrayPackage);

        Self {
            engine,
            ast: Default::default(),
            rhai_feedback: "No issues.".to_string(),
            parsed_code: Default::default(),
        }
    }
}

#[derive(EguiInspect)]
pub struct LivePlot {
    editor: FunctionEditor,
    params: Params,
    plot: FuncPlotter,
    #[inspect(hide)]
    rhai_ctx: RhaiContext,
    #[inspect(hide)]
    zv: Array1<f64>,
}

impl Default for LivePlot {
    fn default() -> Self {
        Self {
            editor: Default::default(),
            params: Default::default(),
            plot: FuncPlotter {
                xy: vec![[0.0, 0.0]; INITIAL_N_POINTS],
            },
            rhai_ctx: Default::default(),
            zv: Array1::linspace(0.0, 1.0, INITIAL_N_POINTS),
        }
    }
}

impl LivePlot {
    fn try_apply_x(&self, z: f64) -> Result<f64, Box<rhai::EvalAltResult>> {
        self.rhai_ctx.engine.call_fn::<f64>(
            &mut Scope::new(),
            &self.rhai_ctx.ast,
            "x",
            (z, self.params.t),
        )
    }
    fn try_apply_y(&self, z: f64) -> Result<f64, Box<rhai::EvalAltResult>> {
        self.rhai_ctx.engine.call_fn::<f64>(
            &mut Scope::new(),
            &self.rhai_ctx.ast,
            "y",
            (z, self.params.t),
        )
    }
    fn update_curve(&mut self) -> Result<(), Box<dyn Error>> {
        // test that functions are appropriately defined
        let _rx = self.try_apply_x(0.5)?;
        let _ry = self.try_apply_y(0.5)?;
        // self.feedback = format!("{_rx} {_ry}");

        // new vec
        self.plot.xy = self
            .zv
            .clone()
            .into_iter()
            .map(|z| [self.try_apply_x(z).unwrap(), self.try_apply_y(z).unwrap()])
            .collect();

        Ok(())
    }
    fn try_run_and_report(&mut self, f: impl Fn(&mut Self) -> Result<(), Box<dyn Error>>) {
        if let Err(e) = f(self) {
            self.rhai_ctx.rhai_feedback = format!("Run error: {e:?}")
        };
    }
}

impl eframe::App for LivePlot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both().show(ui, |ui| {
                if self.params.n_points != self.zv.len() {
                    self.zv = Array1::linspace(0.0, 1.0, self.params.n_points);
                }

                if self.rhai_ctx.parsed_code != self.editor.code {
                    match self.rhai_ctx.engine.compile(self.editor.code.as_str()) {
                        Ok(ast) => {
                            self.rhai_ctx.ast = ast;
                            self.rhai_ctx.rhai_feedback = "No issues.".to_string();
                            if !self.params.advance {
                                self.try_run_and_report(Self::update_curve);
                            }
                        }
                        Err(e) => self.rhai_ctx.rhai_feedback = format!("Parsing error: {e:?}"),
                    }
                    self.rhai_ctx.parsed_code = self.editor.code.clone();
                }

                if self.params.advance || self.params.changed {
                    self.try_run_and_report(Self::update_curve);
                }

                self.rhai_ctx.rhai_feedback.inspect("rhai feedback", ui);
                self.inspect_mut("", ui);
            });
        });
    }
}
