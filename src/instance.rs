use std::sync::Arc;
use std::ffi::{self, CStr};
use std::ptr;
use std::mem;
use std::marker::PhantomData;
use smallvec::SmallVec;
use libc::{c_char, c_void};
use vks;
use ::{VooResult, Loader, ApplicationInfo, ENABLE_VALIDATION_LAYERS};



unsafe extern "system" fn __debug_callback(_flags: vks::VkDebugReportFlagsEXT,
        _obj_type: vks::VkDebugReportObjectTypeEXT, _obj: u64, _location: usize, _code: i32,
        _layer_prefix: *const c_char, msg: *const c_char, _user_data: *mut c_void) -> u32
{
    println!("{}", CStr::from_ptr(msg).to_str().unwrap());
    vks::VK_FALSE
}

// fn create_debug_report_callback_ext(instance: &Instance,
//         create_info: &vks::VkDebugReportCallbackCreateInfoEXT, allocator: vks::VkDebugReportCallbackEXT)
// {
//     let create_drcb = instance.get_instance_proc_addr(b"vkCreateDebugReportCallbackEXT".as_ptr() as *const i8);
// }



pub unsafe fn extension_names<'en>(extensions: &'en [vks::VkExtensionProperties]) -> Vec<&'en CStr> {
    extensions.iter().map(|ext| {
        let name = CStr::from_ptr(&ext.extensionName as *const c_char);
        println!("Enabling instance extension: '{}' (version: {})",
            name.to_str().unwrap(), ext.specVersion);
        name
    }).collect()
}

unsafe fn enumerate_physical_devices(instance: vks::VkInstance, loader: &vks::InstanceProcAddrLoader) -> Vec<vks::VkPhysicalDevice> {
    let mut device_count = 0;
    ::check(loader.core.vkEnumeratePhysicalDevices(instance, &mut device_count, ptr::null_mut()));
    if device_count == 0 { panic!("No physical devices found."); }
    let mut devices = Vec::with_capacity(device_count as usize);
    devices.set_len(device_count as usize);
    ::check(loader.core.vkEnumeratePhysicalDevices(instance, &mut device_count, devices.as_mut_ptr()));
    println!("Available devices: {:?}", devices);
    devices
}


// typedef struct VkInstanceCreateInfo {
//     VkStructureType             sType;
//     const void*                 pNext;
//     VkInstanceCreateFlags       flags;
//     const VkApplicationInfo*    pApplicationInfo;
//     uint32_t                    enabledLayerCount;
//     const char* const*          ppEnabledLayerNames;
//     uint32_t                    enabledExtensionCount;
//     const char* const*          ppEnabledExtensionNames;
// } VkInstanceCreateInfo;


/// A builder used to create an `Instance`.
pub struct InstanceBuilder<'ib> {
    create_info: vks::VkInstanceCreateInfo,
    enabled_layer_name_ptrs: SmallVec<[*const c_char; 128]>,
    enabled_extension_name_ptrs: SmallVec<[*const c_char; 128]>,
    _p: PhantomData<&'ib ()>,
}

impl<'ib> InstanceBuilder<'ib> {
    /// Returns a new instance builder.
    pub fn new() -> InstanceBuilder<'ib> {
        InstanceBuilder {
            create_info: vks::VkInstanceCreateInfo::default(),
            enabled_layer_name_ptrs: SmallVec::new(),
            enabled_extension_name_ptrs: SmallVec::new(),
            _p: PhantomData,
        }
    }

    /// Sets the application info.
    pub fn application_info<'ai, 's>(&'s mut self, application_info: &'ai ApplicationInfo)
            -> &'s mut InstanceBuilder<'ib>
            where 'ai: 'ib {
        self.create_info.pApplicationInfo = application_info.raw();
        self
    }

    /// Sets the enabled layer names.
    pub fn enabled_layer_names<'eln, 's>(&'s mut self, enabled_layer_names: &'eln [&'eln CStr])
            -> &'s mut InstanceBuilder<'ib>
            where 'eln: 'ib {
        for ln in enabled_layer_names {
            self.enabled_layer_name_ptrs.push(ln.as_ptr());
        }
        self.create_info.ppEnabledLayerNames = self.enabled_layer_name_ptrs.as_ptr();
        self.create_info.enabledLayerCount = self.enabled_layer_name_ptrs.len() as u32;
        self
    }

    /// Sets the enabled extension names.
    ///
    /// May not be used with `::enabled_extensions`.
    pub fn enabled_extension_names<'een, 's>(&'s mut self, enabled_extension_names: &'een [&'een CStr])
            -> &'s mut InstanceBuilder<'ib>
            where 'een: 'ib {
        if !self.create_info.ppEnabledExtensionNames.is_null() {
            panic!("Enabled extension names have already been set.");
        }
        for en in enabled_extension_names {
            self.enabled_extension_name_ptrs.push(en.as_ptr());
        }
        self.create_info.ppEnabledExtensionNames = self.enabled_extension_name_ptrs.as_ptr();
        self.create_info.enabledExtensionCount = self.enabled_extension_name_ptrs.len() as u32;
        self
    }

    /// Sets the enabled extension names by providing a list of extensions.
    ///
    /// May not be used with `::enabled_extension_lists`.
    pub fn enabled_extensions<'een, 's>(&'s mut self, enabled_extensions: &'een [vks::VkExtensionProperties])
            -> &'s mut InstanceBuilder<'ib>
            where 'een: 'ib {
        if !self.create_info.ppEnabledExtensionNames.is_null() {
            panic!("Enabled extension names have already been set.");
        }
        for eext in enabled_extensions {
            println!("Enabling instance extension: '{}' (version: {})",
                unsafe { CStr::from_ptr(&eext.extensionName as *const c_char).to_str().unwrap() },
                    eext.specVersion);
            self.enabled_extension_name_ptrs.push(eext.extensionName.as_ptr());
        }
        self.create_info.ppEnabledExtensionNames = self.enabled_extension_name_ptrs.as_ptr();
        self.create_info.enabledExtensionCount = self.enabled_extension_name_ptrs.len() as u32;
        self
    }

    /// Builds and returns an `Instance`.
    pub fn build<'s>(&'s mut self, mut loader: Loader) -> VooResult<Instance> {
        let mut handle = ptr::null_mut();

        unsafe {
            ::check(loader.core_global().vkCreateInstance(&self.create_info, ptr::null(), &mut handle));
            // [FIXME: do this properly] Load extension function pointers:
            loader.loader_mut().load_core(handle);
            loader.loader_mut().load_khr_surface(handle);
            loader.loader_mut().load_khr_win32_surface(handle);
            loader.loader_mut().load_khr_get_physical_device_properties2(handle);
            loader.loader_mut().load_khr_external_memory_capabilities(handle);
        }

        // TODO: Ensure that the debug extension is enabled by consulting the
        // enabled extension list instead.
        if ENABLE_VALIDATION_LAYERS { unsafe { loader.loader_mut().load_ext_debug_report(handle); } }

        // TODO: Ensure that the debug extension is enabled by consulting the
        // enabled extension list instead.
        let debug_callback = if ENABLE_VALIDATION_LAYERS {
            let create_info = vks::VkDebugReportCallbackCreateInfoEXT {
                sType:  vks::VK_STRUCTURE_TYPE_DEBUG_REPORT_CALLBACK_CREATE_INFO_EXT,
                pNext: ptr::null(),
                flags: vks::VK_DEBUG_REPORT_ERROR_BIT_EXT | vks::VK_DEBUG_REPORT_WARNING_BIT_EXT,
                pfnCallback: Some(__debug_callback),
                pUserData: ptr::null_mut(),
            };

            let mut callback: vks::VkDebugReportCallbackEXT = 0;
            if unsafe { loader.loader().ext_debug_report.vkCreateDebugReportCallbackEXT(handle,
                    &create_info, ptr::null(), &mut callback) } != vks::VK_SUCCESS
            {
                panic!("failed to set up debug callback");
            } else {
                println!("Debug report callback initialized.");
            }
            Some(callback)
        } else {
            None
        };

        // Device:
        let physical_devices = unsafe { enumerate_physical_devices(handle, loader.loader()) };

        Ok(Instance {
            inner: Arc::new(Inner {
                handle,
                loader,
                debug_callback,
                physical_devices,
            }),
        })
    }
}


#[derive(Debug)]
struct Inner {
    handle: vks::VkInstance,
    loader: Loader,
    debug_callback: Option<vks::VkDebugReportCallbackEXT>,
    physical_devices: Vec<vks::VkPhysicalDevice>,
}

#[derive(Debug, Clone)]
pub struct Instance {
    inner: Arc<Inner>,
}

impl Instance {
    // // pub unsafe fn new(app_info: &vks::VkApplicationInfo) -> VooResult<Instance> {
    // pub unsafe fn new(app_info: &ApplicationInfo) -> VooResult<Instance> {
    //     let mut loader = Loader::new()?;

    //     // Layers:
    //     let enabled_layer_names = enabled_layer_names(&loader, true);

    //     // Extensions:
    //     let extensions = enumerate_instance_extension_properties(&loader);
    //     let extension_names = extension_names(extensions.as_slice());

    //     // Instance:
    //     let create_info = vks::VkInstanceCreateInfo {
    //         sType: vks::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
    //         pNext: ptr::null(),
    //         flags: 0,
    //         pApplicationInfo: app_info.raw(),
    //         enabledLayerCount: enabled_layer_names.len() as u32,
    //         ppEnabledLayerNames: enabled_layer_names.as_ptr(),
    //         enabledExtensionCount: extension_names.len() as u32,
    //         ppEnabledExtensionNames:extension_names.as_ptr(),
    //     };

    //     let mut handle = ptr::null_mut();
    //     ::check(loader.core_global().vkCreateInstance(&create_info, ptr::null(), &mut handle));
    //     // create_info.enabled_extensions.load_instance(&mut loader, handle); // DACITE WAY

    //     // [FIXME: do this properly] Load extension function pointers:
    //     loader.loader_mut().load_core(handle);
    //     loader.loader_mut().load_khr_surface(handle);
    //     loader.loader_mut().load_khr_win32_surface(handle);
    //     loader.loader_mut().load_khr_get_physical_device_properties2(handle);
    //     loader.loader_mut().load_khr_external_memory_capabilities(handle);
    //     if ENABLE_VALIDATION_LAYERS { loader.loader_mut().load_ext_debug_report(handle); }

    //     let debug_callback = if ENABLE_VALIDATION_LAYERS {
    //         let create_info = vks::VkDebugReportCallbackCreateInfoEXT {
    //             sType:  vks::VK_STRUCTURE_TYPE_DEBUG_REPORT_CALLBACK_CREATE_INFO_EXT,
    //             pNext: ptr::null(),
    //             flags: vks::VK_DEBUG_REPORT_ERROR_BIT_EXT | vks::VK_DEBUG_REPORT_WARNING_BIT_EXT,
    //             pfnCallback: Some(__debug_callback),
    //             pUserData: ptr::null_mut(),
    //         };

    //         let mut callback: vks::VkDebugReportCallbackEXT = 0;
    //         if loader.loader().ext_debug_report.vkCreateDebugReportCallbackEXT(handle,
    //                 &create_info, ptr::null(), &mut callback) != vks::VK_SUCCESS
    //         {
    //             panic!("failed to set up debug callback");
    //         } else {
    //             println!("Debug report callback initialized.");
    //         }
    //         Some(callback)
    //     } else {
    //         None
    //     };

    //     // Device:
    //     let physical_devices = enumerate_physical_devices(handle, loader.loader());

    //     Ok(Instance {
    //         inner: Arc::new(Inner {
    //             handle,
    //             loader,
    //             debug_callback,
    //             physical_devices,
    //         }),
    //     })
    // }

    #[inline]
    pub fn builder<'ib>() -> InstanceBuilder<'ib> {
        InstanceBuilder::new()
    }

    #[inline]
    pub fn vk(&self) -> &vks::InstanceProcAddrLoader {
        self.inner.loader.loader()
    }

    #[inline]
    pub fn handle(&self) -> vks::VkInstance {
        self.inner.handle
    }

    #[inline]
    pub fn get_instance_proc_addr(&self, name: *const i8)
            -> Option<unsafe extern "system" fn(*mut vks::VkInstance_T, *const i8)
                -> Option<unsafe extern "system" fn()>>
    {
        self.inner.loader.get_instance_proc_addr(self.inner.handle, name)
    }

    #[inline]
    pub fn physical_devices(&self) -> &[vks::VkPhysicalDevice] {
        self.inner.physical_devices.as_slice()
    }

    #[inline]
    pub fn loader(&self) -> &Loader {
        &self.inner.loader
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            println!("Destroying debug callback...");
            if let Some(callback) = self.debug_callback {
                self.loader.loader().ext_debug_report.vkDestroyDebugReportCallbackEXT(self.handle, callback, ptr::null());
            }

            println!("Destroying instance...");
            self.loader.loader().core.vkDestroyInstance(self.handle, ptr::null());
        }
    }
}