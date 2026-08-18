//! Безопасная RAII-обертка вокруг конкретного эффекта TensorRT.

use crate::ffi::*;
use crate::loader::NvAFXApi;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::Arc;

pub struct NvAudioEffect {
    api: Arc<NvAFXApi>,
    handle: NvAFXHandle,
    pub frame_size: usize,
}

unsafe impl Send for NvAudioEffect {}
unsafe impl Sync for NvAudioEffect {}

impl NvAudioEffect {
    pub fn new(api: Arc<NvAFXApi>, effect_code: &[u8], sample_rate: u32) -> Result<Self, String> {
        let mut handle: NvAFXHandle = std::ptr::null_mut();

        unsafe {
            let status = (api.create_effect)(effect_code.as_ptr() as *const i8, &mut handle);
            if status != NvAFXStatus::Success || handle.is_null() {
                return Err(format!("Ошибка создания эффекта: {:?}", status));
            }
        }

        let model_path = Self::resolve_model_path(effect_code, sample_rate)
            .ok_or_else(|| "Файл модели .trtpkg не найден".to_string())?;

        let c_model_path = CString::new(model_path.to_string_lossy().as_bytes())
            .map_err(|e| e.to_string())?;

        unsafe {
            // Включаем Voice Activity Detection (VAD)
            let _ = (api.set_u32)(handle, NVAFX_PARAM_ENABLE_VAD.as_ptr() as *const i8, 1);

            // Активируем CUDA Graphs для минимального CPU-оверхеда
            let _ = (api.set_u32)(handle, NVAFX_PARAM_DISABLE_CUDA_GRAPH.as_ptr() as *const i8, 0);

            // Задаем путь к модели
            let status = (api.set_string)(handle, NVAFX_PARAM_MODEL_PATH.as_ptr() as *const i8, c_model_path.as_ptr());
            if status != NvAFXStatus::Success {
                (api.destroy_effect)(handle);
                return Err(format!("Не удалось задать путь к модели: {:?}", status));
            }

            // Загрузка в память GPU (TensorRT Engine compilation / load)
            let status = (api.load)(handle);
            if status != NvAFXStatus::Success {
                (api.destroy_effect)(handle);
                return Err(format!("Ошибка NvAFX_Load: {:?}", status));
            }

            let mut frame_size: u32 = 0;
            let status = (api.get_u32)(handle, NVAFX_PARAM_NUM_INPUT_SAMPLES_PER_FRAME.as_ptr() as *const i8, &mut frame_size);
            if status != NvAFXStatus::Success || frame_size == 0 {
                (api.destroy_effect)(handle);
                return Err("Не удалось получить размер фрейма от SDK".to_string());
            }

            Ok(Self {
                api,
                handle,
                frame_size: frame_size as usize,
            })
        }
    }

    #[inline(always)]
    pub fn set_intensity(&self, intensity: f32) {
        unsafe {
            (self.api.set_float)(self.handle, NVAFX_PARAM_INTENSITY_RATIO.as_ptr() as *const i8, intensity);
        }
    }

    #[inline(always)]
    pub unsafe fn process_frame(&self, in_ptrs: *const *const f32, out_ptrs: *mut *mut f32) -> bool {
        let status = (self.api.run)(
            self.handle,
            in_ptrs,
            out_ptrs,
            self.frame_size as u32,
            1,
        );
        status == NvAFXStatus::Success
    }

    pub fn reset(&self) {
        unsafe {
            (self.api.reset)(self.handle);
        }
    }

    fn resolve_model_path(effect_code: &[u8], sample_rate: u32) -> Option<PathBuf> {
        let rate_str = if sample_rate >= 44100 { "48k" } else { "16k" };
        let prefix = if effect_code == NVAFX_EFFECT_DEREVERB {
            "dereverb"
        } else if effect_code == NVAFX_EFFECT_DEREVERB_DENOISER {
            "dereverb_denoiser"
        } else {
            "denoiser"
        };

        let file_name = format!("{}_{}.trtpkg", prefix, rate_str);

        let mut search_dirs = Vec::new();
        if let Ok(sdk_dir) = std::env::var("NVAFX_SDK_DIR") {
            let p = PathBuf::from(&sdk_dir);
            search_dirs.push(p.join("models"));
            search_dirs.push(p.join("bin").join("models"));
            search_dirs.push(p);
        }
        search_dirs.push(PathBuf::from(r"C:\Program Files\NVIDIA Corporation\NVIDIA Audio Effects\models"));
        search_dirs.push(PathBuf::from(r"C:\Program Files\NVIDIA Corporation\NVIDIA Audio Effects\bin\models"));
        search_dirs.push(PathBuf::from(r"C:\Program Files\NVIDIA Corporation\NVIDIA Broadcast\models"));

        for dir in search_dirs {
            let candidate = dir.join(&file_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }
}

impl Drop for NvAudioEffect {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.api.destroy_effect)(self.handle);
            }
        }
    }
}