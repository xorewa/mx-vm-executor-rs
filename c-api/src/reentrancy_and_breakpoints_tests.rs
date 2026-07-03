use crate::{
    capi_instance::{CapiInstance, vm_exec_instance_destroy, vm_exec_instance_t},
    handle_registry, vm_exec_result_t,
};
use multiversx_chain_vm_executor::{
    BreakpointValueLegacy, ExecutorError, InstanceLegacy, MemLength, MemPtr,
};
use std::sync::mpsc;
use std::time::Duration;

struct MockInstance;

impl InstanceLegacy for MockInstance {
    fn call(&self, _func_name: &str) -> Result<(), String> {
        Ok(())
    }

    fn check_signatures(&self) -> bool {
        true
    }

    fn has_function(&self, _func_name: &str) -> bool {
        true
    }

    fn has_imported_function(&self, _func_name: &str) -> bool {
        true
    }

    fn get_exported_function_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn set_points_limit(&self, _limit: u64) -> Result<(), String> {
        Ok(())
    }

    fn set_points_used(&self, _points: u64) -> Result<(), String> {
        Ok(())
    }

    fn get_points_used(&self) -> Result<u64, String> {
        Ok(0)
    }

    fn memory_length(&self) -> Result<u64, String> {
        Ok(0)
    }

    fn memory_ptr(&self) -> Result<*mut u8, String> {
        Ok(std::ptr::null_mut())
    }

    fn memory_load(
        &self,
        _mem_ptr: MemPtr,
        _mem_length: MemLength,
    ) -> Result<&[u8], ExecutorError> {
        Ok(&[])
    }

    fn memory_store(&self, _mem_ptr: MemPtr, _data: &[u8]) -> Result<(), ExecutorError> {
        Ok(())
    }

    fn memory_grow(&self, _by_num_pages: u32) -> Result<u32, ExecutorError> {
        Ok(0)
    }

    fn set_breakpoint_value(&self, _value: BreakpointValueLegacy) -> Result<(), String> {
        Ok(())
    }

    fn get_breakpoint_value(&self) -> Result<BreakpointValueLegacy, String> {
        Ok(BreakpointValueLegacy::None)
    }

    fn reset(&self) -> Result<(), String> {
        Ok(())
    }

    fn cache(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
}

fn new_mock_instance_ptr() -> *mut vm_exec_instance_t {
    let id = handle_registry::register_instance(CapiInstance::new(Box::new(MockInstance)));
    id as *mut vm_exec_instance_t
}

#[test]
fn c_api_memory_operation_allows_same_thread_reentry() {
    let instance_ptr = new_mock_instance_ptr();
    let capi_instance =
        handle_registry::lookup_instance(instance_ptr as u64).expect("instance registered");
    let _gate = capi_instance.enter_operation();

    let length = unsafe { crate::capi_memory::vm_exec_instance_memory_data_length(instance_ptr) };

    assert_eq!(length, 0);
    unsafe { vm_exec_instance_destroy(instance_ptr) };
}

#[test]
fn c_api_memory_operation_waits_behind_cross_thread_gate_owner() {
    let instance_ptr = new_mock_instance_ptr();
    let capi_instance =
        handle_registry::lookup_instance(instance_ptr as u64).expect("instance registered");
    let gate = capi_instance.enter_operation();
    let (completed_tx, completed_rx) = mpsc::channel();
    let instance_id = instance_ptr as usize;

    let handle = std::thread::spawn(move || {
        let instance_ptr = instance_id as *mut vm_exec_instance_t;
        let length =
            unsafe { crate::capi_memory::vm_exec_instance_memory_data_length(instance_ptr) };
        completed_tx.send(length).unwrap();
    });

    assert!(
        completed_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "ordinary memory access must wait while another thread owns the instance gate"
    );

    drop(gate);
    let length = completed_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("memory operation did not resume after gate release");
    assert_eq!(length, 0);
    handle.join().unwrap();
    unsafe { vm_exec_instance_destroy(instance_ptr) };
}

#[test]
fn c_api_breakpoint_get_bypasses_gate_for_runtime_observation() {
    let instance_ptr = new_mock_instance_ptr();
    let capi_instance =
        handle_registry::lookup_instance(instance_ptr as u64).expect("instance registered");
    let gate = capi_instance.enter_operation();
    let (completed_tx, completed_rx) = mpsc::channel();
    let instance_id = instance_ptr as usize;

    let handle = std::thread::spawn(move || {
        let instance_ptr = instance_id as *const vm_exec_instance_t;
        let value =
            unsafe { crate::capi_breakpoints::vm_exec_instance_get_breakpoint_value(instance_ptr) };
        completed_tx.send(value).unwrap();
    });

    let value = completed_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("breakpoint get blocked behind the operation gate");
    assert_eq!(value, BreakpointValueLegacy::None.as_u64());

    drop(gate);
    handle.join().unwrap();
    unsafe { vm_exec_instance_destroy(instance_ptr) };
}

#[test]
fn c_api_breakpoint_set_bypasses_gate_for_timeout_interrupts() {
    let instance_ptr = new_mock_instance_ptr();
    let capi_instance =
        handle_registry::lookup_instance(instance_ptr as u64).expect("instance registered");
    let gate = capi_instance.enter_operation();
    let (completed_tx, completed_rx) = mpsc::channel();
    let instance_id = instance_ptr as usize;

    let handle = std::thread::spawn(move || {
        let instance_ptr = instance_id as *const vm_exec_instance_t;
        let result = unsafe {
            crate::capi_breakpoints::vm_exec_instance_set_breakpoint_value(instance_ptr, 4)
        };
        completed_tx.send(result as u32).unwrap();
    });

    let result = completed_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("breakpoint set blocked behind the operation gate");
    assert_eq!(result, vm_exec_result_t::VM_EXEC_OK as u32);

    drop(gate);
    handle.join().unwrap();
    unsafe { vm_exec_instance_destroy(instance_ptr) };
}
