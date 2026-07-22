#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, status_message)]
        type AppBootstrap = super::AppBootstrapRust;

        #[qinvokable]
        fn ping(&self) -> QString;
    }
}

use cxx_qt_lib::QString;

pub struct AppBootstrapRust {
    status_message: QString,
}

impl qobject::AppBootstrap {
    fn ping(&self) -> QString {
        if self.status_message().is_empty() {
            QString::from("Rust <-> QML bridge ready")
        } else {
            self.status_message().clone()
        }
    }
}

impl Default for AppBootstrapRust {
    fn default() -> Self {
        Self {
            status_message: QString::from("Rust <-> QML bridge ready"),
        }
    }
}
