//! Сырые FFI-биндинги к NVIDIA Maxine Audio Effects (AFX) C API.

#![allow(dead_code)]

use std::os::raw::{c_char, c_float, c_int, c_uint, c_void};

pub type NvAFXHandle = *mut c_void;
pub type NvAFXEffectSelector = *const c_char;
pub type NvAFXParameterSelector = *const c_char;

/// Статусы возврата функций NVIDIA Audio Effects SDK
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NvAFXStatus {
    Success = 0,
    Failed = 1,
    InvalidHandle = 2,
    InvalidParam = 3,
    ImmutableParam = 4,
    InsufficientData = 5,
    EffectNotAvailable = 6,
    OutputBufferTooSmall = 7,
    ModelLoadFailed = 8,
    Status32ServerNotRegistered = 9,
    Status32ComError = 10,
    GpuUnsupported = 11,
    CudaContextCreationFailed = 12,
}

// Селекторы эффектов
pub const NVAFX_EFFECT_DENOISER: &[u8] = b"denoiser\0";
pub const NVAFX_EFFECT_DEREVERB: &[u8] = b"dereverb\0";
pub const NVAFX_EFFECT_DEREVERB_DENOISER: &[u8] = b"dereverb_denoiser\0";

// Селекторы параметров
pub const NVAFX_PARAM_MODEL_PATH: &[u8] = b"model_path\0";
pub const NVAFX_PARAM_INPUT_SAMPLE_RATE: &[u8] = b"input_sample_rate\0";
pub const NVAFX_PARAM_OUTPUT_SAMPLE_RATE: &[u8] = b"output_sample_rate\0";
pub const NVAFX_PARAM_NUM_INPUT_SAMPLES_PER_FRAME: &[u8] = b"num_input_samples_per_frame\0";
pub const NVAFX_PARAM_NUM_OUTPUT_SAMPLES_PER_FRAME: &[u8] = b"num_output_samples_per_frame\0";
pub const NVAFX_PARAM_NUM_INPUT_CHANNELS: &[u8] = b"num_input_channels\0";
pub const NVAFX_PARAM_NUM_OUTPUT_CHANNELS: &[u8] = b"num_output_channels\0";
pub const NVAFX_PARAM_INTENSITY_RATIO: &[u8] = b"intensity_ratio\0";
pub const NVAFX_PARAM_ENABLE_VAD: &[u8] = b"enable_vad\0";
pub const NVAFX_PARAM_VAD_RESULT: &[u8] = b"vad_result\0";
pub const NVAFX_PARAM_DISABLE_CUDA_GRAPH: &[u8] = b"disable_cuda_graph\0";

// Сигнатуры функций библиотеки
pub type FnNvAFXCreateEffect = unsafe extern "C" fn(code: NvAFXEffectSelector, effect: *mut NvAFXHandle) -> NvAFXStatus;
pub type FnNvAFXDestroyEffect = unsafe extern "C" fn(effect: NvAFXHandle) -> NvAFXStatus;
pub type FnNvAFXSetU32 = unsafe extern "C" fn(effect: NvAFXHandle, param_name: NvAFXParameterSelector, val: c_uint) -> NvAFXStatus;
pub type FnNvAFXSetString = unsafe extern "C" fn(effect: NvAFXHandle, param_name: NvAFXParameterSelector, val: *const c_char) -> NvAFXStatus;
pub type FnNvAFXSetFloat = unsafe extern "C" fn(effect: NvAFXHandle, param_name: NvAFXParameterSelector, val: c_float) -> NvAFXStatus;
pub type FnNvAFXGetU32 = unsafe extern "C" fn(effect: NvAFXHandle, param_name: NvAFXParameterSelector, val: *mut c_uint) -> NvAFXStatus;
pub type FnNvAFXGetBoolList = unsafe extern "C" fn(
    effect: NvAFXHandle,
    param_name: NvAFXParameterSelector,
    list: *mut u8,
    list_size: *mut c_int,
) -> NvAFXStatus;
pub type FnNvAFXLoad = unsafe extern "C" fn(effect: NvAFXHandle) -> NvAFXStatus;
pub type FnNvAFXRun = unsafe extern "C" fn(
    effect: NvAFXHandle,
    input: *const *const c_float,
    output: *mut *mut c_float,
    num_input_samples: c_uint,
    num_input_channels: c_uint,
) -> NvAFXStatus;
pub type FnNvAFXReset = unsafe extern "C" fn(effect: NvAFXHandle) -> NvAFXStatus;