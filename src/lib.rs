//! # NVIDIA Maxine Audio FX VST3 Plugin
//! Разработчик: RinLogs

mod ffi;
mod loader;
mod ring_buffer;
mod wrapper;

use ffi::{NVAFX_EFFECT_DENOISER, NVAFX_EFFECT_DEREVERB, NVAFX_EFFECT_DEREVERB_DENOISER};
use loader::NvAFXApi;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use ring_buffer::FixedRingBuffer;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wrapper::NvAudioEffect;

// Фирменная палитра NVIDIA
const NV_GREEN: egui::Color32 = egui::Color32::from_rgb(118, 185, 0);       // #76B900 (GeForce Green)
const NV_GREEN_GLOW: egui::Color32 = egui::Color32::from_rgb(0, 255, 128);  // Неоновый зеленый LED
const BG_DARK: egui::Color32 = egui::Color32::from_rgb(18, 20, 24);         // Фон плагина
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(26, 29, 35);         // Карточки
const CARD_BORDER: egui::Color32 = egui::Color32::from_rgb(44, 48, 58);

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug)]
pub enum EffectMode {
    #[name = "Noise Suppression"]
    Denoise,
    #[name = "Room Echo Removal"]
    Dereverb,
    #[name = "Denoise + De-Echo"]
    DereverbDenoise,
}

pub struct NvAudioFxPlugin {
    params: Arc<NvAudioFxParams>,
    _api: Option<Arc<NvAFXApi>>,

    effect_denoise: Option<NvAudioEffect>,
    effect_dereverb: Option<NvAudioEffect>,
    effect_combo: Option<NvAudioEffect>,

    in_ring: FixedRingBuffer,
    out_ring: FixedRingBuffer,

    scratch_in: Vec<f32>,
    scratch_out: Vec<f32>,

    last_intensity: f32,
    frame_size: usize,
    sample_rate: f32,
    initialized_successfully: bool,
    speech_hold_counter: usize,
    voice_active: Arc<AtomicBool>,
    status_message: Arc<Mutex<String>>,
}

#[derive(Params)]
pub struct NvAudioFxParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "mode"]
    pub mode: EnumParam<EffectMode>,

    #[id = "intensity"]
    pub intensity: FloatParam,
}

impl Default for NvAudioFxPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(NvAudioFxParams {
                editor_state: EguiState::from_size(480, 380),
                mode: EnumParam::new("Mode", EffectMode::Denoise),
                intensity: FloatParam::new(
                    "Intensity",
                    1.0,
                    FloatRange::Linear { min: 0.0, max: 1.0 },
                )
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0)),
            }),
            _api: None,
            effect_denoise: None,
            effect_dereverb: None,
            effect_combo: None,
            in_ring: FixedRingBuffer::new(8192),
            out_ring: FixedRingBuffer::new(8192),
            scratch_in: Vec::new(),
            scratch_out: Vec::new(),
            last_intensity: -1.0,
            frame_size: 480,
            sample_rate: 48000.0,
            initialized_successfully: false,
            speech_hold_counter: 0,
            voice_active: Arc::new(AtomicBool::new(false)),
            status_message: Arc::new(Mutex::new("Starting...".into())),
        }
    }
}

impl Plugin for NvAudioFxPlugin {
    const NAME: &'static str = "NVIDIA Maxine Audio FX";
    const VENDOR: &'static str = "RinLogs";
    const URL: &'static str = "https://github.com/RinLogs";
    const EMAIL: &'static str = "rinseon.exe@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let status_lock = self.status_message.clone();
        let voice_active = self.voice_active.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                let mut visuals = egui::Visuals::dark();
                visuals.override_text_color = Some(egui::Color32::from_rgb(230, 235, 245));
                visuals.widgets.active.bg_fill = NV_GREEN;
                visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(42, 47, 58);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(32, 36, 46);
                egui_ctx.set_visuals(visuals);

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(BG_DARK).inner_margin(20.0))
                    .show(egui_ctx, |ui| {
                        // 1. HEADER
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("NVIDIA").strong().size(24.0).color(NV_GREEN));
                            ui.label(egui::RichText::new("MAXINE").strong().size(24.0).color(egui::Color32::WHITE));
                            ui.label(egui::RichText::new("• AUDIO FX").size(16.0).color(egui::Color32::from_rgb(150, 155, 170)));
                        });

                        ui.add_space(14.0);

                        // 2. STATUS & VAD CARD
                        egui::Frame::none()
                            .fill(CARD_BG)
                            .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                            .rounding(10.0)
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let status = status_lock.lock().unwrap().clone();
                                    let (gpu_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                    let is_ok = status.contains("OK");
                                    ui.painter().circle_filled(
                                        gpu_rect.center(),
                                        6.0,
                                        if is_ok { NV_GREEN } else { egui::Color32::from_rgb(255, 70, 70) },
                                    );
                                    ui.label(egui::RichText::new("RTX TensorRT").size(14.0).strong());

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let is_speaking = voice_active.load(Ordering::Relaxed);
                                        let (vad_rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                                        let center = vad_rect.center();

                                        if is_speaking {
                                            ui.painter().circle_filled(center, 9.0, egui::Color32::from_rgba_unmultiplied(0, 255, 128, 80));
                                            ui.painter().circle_filled(center, 5.5, NV_GREEN_GLOW);
                                            ui.label(egui::RichText::new("VOICE ACTIVE").color(NV_GREEN_GLOW).size(14.0).strong());
                                        } else {
                                            ui.painter().circle_filled(center, 5.5, egui::Color32::from_rgb(55, 62, 72));
                                            ui.label(egui::RichText::new("SILENCE / NOISE").color(egui::Color32::from_rgb(120, 125, 140)).size(14.0));
                                        }
                                    });
                                });
                            });

                        ui.add_space(16.0);

                        // 3. EFFECT MODE SELECTOR (Segmented Buttons)
                        ui.label(egui::RichText::new("EFFECT MODE").size(13.0).color(egui::Color32::from_rgb(160, 165, 180)).strong());
                        ui.add_space(6.0);

                        let current_mode = params.mode.value();
                        ui.horizontal(|ui| {
                            let modes = [
                                (EffectMode::Denoise, "Noise Denoise"),
                                (EffectMode::Dereverb, "Room De-Echo"),
                                (EffectMode::DereverbDenoise, "Denoise + Echo"),
                            ];

                            let total_width = ui.available_width();
                            let button_width = (total_width - 12.0) / 3.0;

                            for (mode, title) in modes {
                                let is_active = current_mode == mode;
                                let btn = egui::Button::new(
                                    egui::RichText::new(title)
                                        .size(14.0)
                                        .strong()
                                        .color(if is_active { egui::Color32::BLACK } else { egui::Color32::WHITE }),
                                )
                                .fill(if is_active { NV_GREEN } else { CARD_BG })
                                .stroke(egui::Stroke::new(1.2, if is_active { NV_GREEN } else { CARD_BORDER }))
                                .rounding(8.0)
                                .min_size(egui::vec2(button_width, 42.0));

                                if ui.add(btn).clicked() {
                                    setter.set_parameter(&params.mode, mode);
                                }
                            }
                        });

                        ui.add_space(18.0);

                        // 4. SUPPRESSION INTENSITY CARD
                        egui::Frame::none()
                            .fill(CARD_BG)
                            .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                            .rounding(10.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                let mut current_intensity = params.intensity.value();

                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("SUPPRESSION INTENSITY").size(13.0).color(egui::Color32::from_rgb(160, 165, 180)).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}%", current_intensity * 100.0))
                                                .color(NV_GREEN)
                                                .strong()
                                                .size(24.0),
                                        );
                                    });
                                });

                                ui.add_space(10.0);

                                let slider = egui::Slider::new(&mut current_intensity, 0.0..=1.0)
                                    .show_value(false)
                                    .trailing_fill(true);

                                if ui.add_sized([ui.available_width(), 26.0], slider).changed() {
                                    setter.set_parameter(&params.intensity, current_intensity);
                                }

                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("0% (Passthrough)").size(12.0).color(egui::Color32::from_rgb(100, 105, 120)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new("100% (Aggressive)").size(12.0).color(egui::Color32::from_rgb(100, 105, 120)));
                                    });
                                });
                            });

                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Powered by NVIDIA Maxine • Plugin by RinLogs")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(110, 115, 130)),
                            );
                        });
                    });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.initialized_successfully = false;

        let api = match NvAFXApi::load_from_system() {
            Ok(api) => api,
            Err(e) => {
                if let Ok(mut status) = self.status_message.lock() {
                    *status = format!("DLL Error: {}", e);
                }
                return true;
            }
        };

        let sample_rate_u32 = self.sample_rate as u32;

        self.effect_denoise = NvAudioEffect::new(api.clone(), NVAFX_EFFECT_DENOISER, sample_rate_u32).ok();
        self.effect_dereverb = NvAudioEffect::new(api.clone(), NVAFX_EFFECT_DEREVERB, sample_rate_u32).ok();
        self.effect_combo = NvAudioEffect::new(api.clone(), NVAFX_EFFECT_DEREVERB_DENOISER, sample_rate_u32).ok();

        if let Some(ref eff) = self.effect_denoise {
            self.frame_size = eff.frame_size;
        } else if let Some(ref eff) = self.effect_dereverb {
            self.frame_size = eff.frame_size;
        } else if let Some(ref eff) = self.effect_combo {
            self.frame_size = eff.frame_size;
        } else {
            if let Ok(mut status) = self.status_message.lock() {
                *status = "Model Load Failed".into();
            }
            return true;
        }

        self.scratch_in = vec![0.0; self.frame_size];
        self.scratch_out = vec![0.0; self.frame_size];

        self.in_ring.clear();
        self.out_ring.clear();

        let silence = vec![0.0; self.frame_size];
        self.out_ring.push_slice(&silence);

        self._api = Some(api);
        self.last_intensity = -1.0;
        self.speech_hold_counter = 0;
        self.initialized_successfully = true;

        if let Ok(mut status) = self.status_message.lock() {
            *status = "OK (TensorRT / CUDA Graphs)".into();
        }

        context.set_latency_samples(self.frame_size as u32);
        true
    }

    fn reset(&mut self) {
        if let Some(eff) = &self.effect_denoise {
            eff.reset();
        }
        if let Some(eff) = &self.effect_dereverb {
            eff.reset();
        }
        if let Some(eff) = &self.effect_combo {
            eff.reset();
        }
        self.in_ring.clear();
        self.out_ring.clear();

        let silence = vec![0.0; self.frame_size];
        self.out_ring.push_slice(&silence);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if !self.initialized_successfully {
            self.voice_active.store(false, Ordering::Relaxed);
            return ProcessStatus::Normal;
        }

        let active_effect = match self.params.mode.value() {
            EffectMode::Denoise => self.effect_denoise.as_ref(),
            EffectMode::Dereverb => self.effect_dereverb.as_ref().or(self.effect_denoise.as_ref()),
            EffectMode::DereverbDenoise => self.effect_combo.as_ref().or(self.effect_denoise.as_ref()),
        };

        // Нелинейная кривая x^1.7: мягкая регулировка до 50% и крутой подъем к 100%
        let raw_intensity = self.params.intensity.value();
        let curved_intensity = raw_intensity.powf(1.7);

        if let Some(eff) = active_effect {
            if (curved_intensity - self.last_intensity).abs() > 0.001 {
                eff.set_intensity(curved_intensity);
                self.last_intensity = curved_intensity;
            }

            let num_samples = buffer.samples();
            let channels = buffer.as_slice();
            let (l_channel, r_channel) = channels.split_at_mut(1);
            let in_l = &mut l_channel[0];
            let in_r = &mut r_channel[0];

            for i in 0..num_samples {
                let mono = (in_l[i] + in_r[i]) * 0.5;
                self.in_ring.push_slice(&[mono]);
            }

            let mut speech_detected_in_block = false;

            while self.in_ring.available_samples() >= self.frame_size {
                self.in_ring.read_chunk(&mut self.scratch_in);

                let in_ptr = [self.scratch_in.as_ptr()];
                let out_ptr = [self.scratch_out.as_mut_ptr()];

                unsafe {
                    eff.process_frame(in_ptr.as_ptr(), out_ptr.as_ptr() as *mut *mut f32);
                }

                // Расчет энергии очищенного сигнала для VAD
                let mut sum_sq = 0.0f32;
                for &sample in &self.scratch_out {
                    sum_sq += sample * sample;
                }
                let rms = (sum_sq / self.frame_size as f32).sqrt();

                if rms > 0.004 {
                    speech_detected_in_block = true;
                }

                self.out_ring.push_slice(&self.scratch_out);
            }

            if speech_detected_in_block {
                self.speech_hold_counter = 18; // 180 мс удержание LED
            } else if self.speech_hold_counter > 0 {
                self.speech_hold_counter -= 1;
            }

            self.voice_active.store(self.speech_hold_counter > 0, Ordering::Relaxed);

            for i in 0..num_samples {
                let mut out_sample = [0.0];
                if self.out_ring.read_chunk(&mut out_sample) {
                    in_l[i] = out_sample[0];
                    in_r[i] = out_sample[0];
                } else {
                    in_l[i] = 0.0;
                    in_r[i] = 0.0;
                }
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for NvAudioFxPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"NvMaxineDenoise!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Restoration];
}

nih_export_vst3!(NvAudioFxPlugin);