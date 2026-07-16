use crate::error::{PerchError, Result};
use std::ffi::OsString;
use windows_service::service::{
    ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

pub mod runner;

const SERVICE_NAME: &str = "Perch";
const SERVICE_DESCRIPTION: &str =
    "Perch 轻量级进程与服务管理器:统一看管后台进程,支持崩溃自动重启、定时任务与内置 Web 管理界面(默认 http://127.0.0.1:9000)。";

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
    // 需要 CHANGE_CONFIG 权限才能设置服务描述(SetServiceDescription)。
    let service = mgr
        .create_service(&info, ServiceAccess::CHANGE_CONFIG)
        .map_err(|e| PerchError::Service(format!("create: {e}")))?;
    // 服务描述:显示在“服务”管理器的“描述”栏,便于识别此服务的用途。
    service
        .set_description(SERVICE_DESCRIPTION)
        .map_err(|e| PerchError::Service(format!("set description: {e}")))?;
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
