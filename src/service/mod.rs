use crate::error::{PerchError, Result};
use std::ffi::OsString;
use windows_service::service::{
    ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

pub mod runner;

const SERVICE_NAME: &str = "Perch";

fn manager(access: ServiceManagerAccess) -> Result<ServiceManager> {
    ServiceManager::local_computer(None::<&str>, access)
        .map_err(|e| PerchError::Service(format!("open SCM: {e}")))
}

pub fn install() -> Result<()> {
    let mgr = manager(ServiceManagerAccess::CREATE_SERVICE)?;
    let exe = std::env::current_exe().map_err(|e| PerchError::Service(format!("exe path: {e}")))?;
    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("Perch Process Manager"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        launch_arguments: vec![OsString::from("service")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };
    mgr.create_service(&info, ServiceAccess::empty())
        .map_err(|e| PerchError::Service(format!("create: {e}")))?;
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let mgr = manager(ServiceManagerAccess::CONNECT)?;
    let svc = mgr
        .open_service(SERVICE_NAME, ServiceAccess::DELETE)
        .map_err(|e| PerchError::Service(format!("open: {e}")))?;
    svc.delete().map_err(|e| PerchError::Service(format!("delete: {e}")))?;
    Ok(())
}

pub fn start() -> Result<()> {
    let mgr = manager(ServiceManagerAccess::CONNECT)?;
    let svc = mgr
        .open_service(SERVICE_NAME, ServiceAccess::START)
        .map_err(|e| PerchError::Service(format!("open: {e}")))?;
    svc.start::<OsString>(&[])
        .map_err(|e| PerchError::Service(format!("start: {e}")))?;
    Ok(())
}

pub fn stop() -> Result<()> {
    let mgr = manager(ServiceManagerAccess::CONNECT)?;
    let svc = mgr
        .open_service(SERVICE_NAME, ServiceAccess::STOP)
        .map_err(|e| PerchError::Service(format!("open: {e}")))?;
    svc.stop().map_err(|e| PerchError::Service(format!("stop: {e}")))?;
    Ok(())
}
