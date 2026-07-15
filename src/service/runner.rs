use std::ffi::OsString;
use std::time::Duration;
use tokio::sync::oneshot;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

const SERVICE_NAME: &str = "Perch";

windows_service::define_windows_service!(ffi_service_main, service_main);

pub fn run_as_service() -> Result<(), windows_service::Error> {
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn service_main(_args: Vec<OsString>) {
    let (tx, rx) = oneshot::channel::<()>();
    let mut tx = Some(tx);

    let status_handle = service_control_handler::register(SERVICE_NAME, move |control| match control {
        ServiceControl::Stop => {
            if let Some(t) = tx.take() {
                let _ = t.send(());
            }
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    })
    .expect("register handler");

    let status = |state| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    status_handle.set_service_status(status(ServiceState::Running)).ok();

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let _ = crate::cli::run_foreground_until(async {
            let _ = rx.await;
        })
        .await;
    });

    status_handle.set_service_status(status(ServiceState::Stopped)).ok();
}
