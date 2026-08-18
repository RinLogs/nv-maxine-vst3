//! Модуль безопасной динамической загрузки NVAudioEffects.dll без жесткой линковки.

use crate::ffi::*;
use libloading::{Library, Symbol};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

extern "system" {
    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
}

pub struct NvAFXApi {
    _lib: Library,
    pub create_effect: FnNvAFXCreateEffect,
    pub destroy_effect: FnNvAFXDestroyEffect,
    pub set_u32: FnNvAFXSetU32,
    pub set_string: FnNvAFXSetString,
    pub set_float: FnNvAFXSetFloat,
    pub get_u32: FnNvAFXGetU32,
    pub get_bool_list: Option<FnNvAFXGetBoolList>,
    pub load: FnNvAFXLoad,
    pub run: FnNvAFXRun,
    pub reset: FnNvAFXReset,
}

impl NvAFXApi {
    /// Поиск и загрузка NVAudioEffects.dll с настройкой путей зависимостей CUDA/TensorRT
    pub fn load_from_system() -> Result<Arc<Self>, String> {
        let dll_path = Self::find_dll_path()
            .ok_or_else(|| "NVAudioEffects.dll не найдена. Убедитесь, что установлен NVIDIA Broadcast SDK.".to_string())?;

        // Добавляем директорию с DLL в список поиска DLL зависимостей Windows
        if let Some(dir) = dll_path.parent() {
            let mut wide_dir: Vec<u16> = OsStr::new(dir).encode_wide().collect();
            wide_dir.push(0);
            unsafe {
                SetDllDirectoryW(wide_dir.as_ptr());
            }
        }

        unsafe {
            let lib = Library::new(&dll_path)
                .map_err(|e| format!("Не удалось загрузить DLL {:?}: {}", dll_path, e))?;

            let create_effect: Symbol<FnNvAFXCreateEffect> = lib.get(b"NvAFX_CreateEffect\0").map_err(|e| e.to_string())?;
            let destroy_effect: Symbol<FnNvAFXDestroyEffect> = lib.get(b"NvAFX_DestroyEffect\0").map_err(|e| e.to_string())?;
            let set_u32: Symbol<FnNvAFXSetU32> = lib.get(b"NvAFX_SetU32\0").map_err(|e| e.to_string())?;
            let set_string: Symbol<FnNvAFXSetString> = lib.get(b"NvAFX_SetString\0").map_err(|e| e.to_string())?;
            let set_float: Symbol<FnNvAFXSetFloat> = lib.get(b"NvAFX_SetFloat\0").map_err(|e| e.to_string())?;
            let get_u32: Symbol<FnNvAFXGetU32> = lib.get(b"NvAFX_GetU32\0").map_err(|e| e.to_string())?;
            let get_bool_list: Option<FnNvAFXGetBoolList> = lib.get(b"NvAFX_GetBoolList\0").ok().map(|s| *s);
            let load: Symbol<FnNvAFXLoad> = lib.get(b"NvAFX_Load\0").map_err(|e| e.to_string())?;
            let run: Symbol<FnNvAFXRun> = lib.get(b"NvAFX_Run\0").map_err(|e| e.to_string())?;
            let reset: Symbol<FnNvAFXReset> = lib.get(b"NvAFX_Reset\0").map_err(|e| e.to_string())?;

            Ok(Arc::new(Self {
                create_effect: *create_effect,
                destroy_effect: *destroy_effect,
                set_u32: *set_u32,
                set_string: *set_string,
                set_float: *set_float,
                get_u32: *get_u32,
                get_bool_list,
                load: *load,
                run: *run,
                reset: *reset,
                _lib: lib,
            }))
        }
    }

    fn find_dll_path() -> Option<PathBuf> {
        if let Ok(sdk_dir) = std::env::var("NVAFX_SDK_DIR") {
            let path = Path::new(&sdk_dir).join("NVAudioEffects.dll");
            if path.exists() {
                return Some(path);
            }
            let bin_path = Path::new(&sdk_dir).join("bin").join("NVAudioEffects.dll");
            if bin_path.exists() {
                return Some(bin_path);
            }
        }

        let standard_paths = [
            r"C:\Program Files\NVIDIA Corporation\NVIDIA Audio Effects\NVAudioEffects.dll",
            r"C:\Program Files\NVIDIA Corporation\NVIDIA Audio Effects\bin\NVAudioEffects.dll",
            r"C:\Program Files\NVIDIA Corporation\NVIDIA Broadcast\NVAudioEffects.dll",
            r"C:\Program Files\NVIDIA Corporation\NVIDIA Broadcast\bin\NVAudioEffects.dll",
        ];

        for p in &standard_paths {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }
}