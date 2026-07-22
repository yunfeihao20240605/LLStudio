//! QML adapter for the persistent video learning library.

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::path::Path;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, in_progress_count, cxx_name = "inProgressCount")]
        #[qproperty(i32, ungrouped_video_count, cxx_name = "ungroupedVideoCount")]
        #[qproperty(i32, list_count, cxx_name = "listCount")]
        #[qproperty(i32, revision)]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type LibraryBridge = super::LibraryBridgeRust;

        #[qinvokable]
        #[cxx_name = "recordOpenedVideo"]
        fn record_opened_video(
            self: Pin<&mut LibraryBridge>,
            path: &QString,
            duration_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "refresh"]
        fn refresh(self: Pin<&mut LibraryBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "videoTitleAt"]
        fn video_title_at(&self, index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "videoPathAt"]
        fn video_path_at(&self, index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "videoDurationAt"]
        fn video_duration_at(&self, index: i32) -> f64;

        #[qinvokable]
        #[cxx_name = "createList"]
        fn create_list(self: Pin<&mut LibraryBridge>, name: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteList"]
        fn delete_list(self: Pin<&mut LibraryBridge>, list_index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "moveVideoToList"]
        fn move_video_to_list(
            self: Pin<&mut LibraryBridge>,
            path: &QString,
            list_index: i32,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "removeVideo"]
        fn remove_video(self: Pin<&mut LibraryBridge>, path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "listNameAt"]
        fn list_name_at(&self, list_index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "listVideoCountAt"]
        fn list_video_count_at(&self, list_index: i32) -> i32;

        #[qinvokable]
        #[cxx_name = "listVideoTitleAt"]
        fn list_video_title_at(&self, list_index: i32, video_index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "listVideoPathAt"]
        fn list_video_path_at(&self, list_index: i32, video_index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "ungroupedVideoTitleAt"]
        fn ungrouped_video_title_at(&self, video_index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "ungroupedVideoPathAt"]
        fn ungrouped_video_path_at(&self, video_index: i32) -> QString;
    }
}

pub struct LibraryBridgeRust {
    in_progress_count: i32,
    ungrouped_video_count: i32,
    list_count: i32,
    revision: i32,
    status_message: QString,
    repository: els_storage::VideoLibraryRepository,
    videos: Vec<els_storage::LearningVideo>,
    lists: Vec<els_storage::VideoList>,
}

impl Default for LibraryBridgeRust {
    fn default() -> Self {
        let repository = match els_storage::VideoLibraryRepository::open_default() {
            Ok(repository) => repository,
            Err(err) => {
                let message = format!("初始化学习库失败：{err}");
                eprintln!("{message}");
                return Self {
                    in_progress_count: 0,
                    ungrouped_video_count: 0,
                    list_count: 0,
                    revision: 1,
                    status_message: QString::from(&message),
                    repository: els_storage::VideoLibraryRepository::disabled(),
                    videos: Vec::new(),
                    lists: Vec::new(),
                };
            }
        };
        let (videos, lists, status_message) =
            match (repository.list_in_progress(), repository.list_video_lists()) {
                (Ok(videos), Ok(lists)) => (videos, lists, QString::from("学习库已加载")),
                (Err(err), _) | (_, Err(err)) => {
                    let message = format!("读取学习库失败：{err}");
                    eprintln!("{message}");
                    (Vec::new(), Vec::new(), QString::from(&message))
                }
            };

        let ungrouped_count = videos
            .iter()
            .filter(|video| video.list_id.is_none())
            .count();

        Self {
            in_progress_count: videos.len().min(i32::MAX as usize) as i32,
            ungrouped_video_count: ungrouped_count.min(i32::MAX as usize) as i32,
            list_count: lists.len().min(i32::MAX as usize) as i32,
            revision: 1,
            status_message,
            repository,
            videos,
            lists,
        }
    }
}

impl qobject::LibraryBridge {
    fn record_opened_video(mut self: Pin<&mut Self>, path: &QString, duration_secs: f64) -> bool {
        let path = path.to_string();
        if path.trim().is_empty() {
            self.as_mut()
                .set_status_message(QString::from("视频路径不能为空"));
            return false;
        }
        let title = Path::new(&path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&path)
            .to_string();
        let result = self.as_mut().rust_mut().repository.record_opened(
            &path,
            &title,
            duration_secs.max(0.0),
        );
        if let Err(err) = result {
            let message = format!("保存学习视频失败：{err}");
            eprintln!("{message}");
            self.as_mut().set_status_message(QString::from(&message));
            return false;
        }

        self.as_mut().refresh()
    }

    fn refresh(mut self: Pin<&mut Self>) -> bool {
        let (videos, lists) = match (
            self.rust().repository.list_in_progress(),
            self.rust().repository.list_video_lists(),
        ) {
            (Ok(videos), Ok(lists)) => (videos, lists),
            (Err(err), _) | (_, Err(err)) => {
                let message = format!("刷新学习库失败：{err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                return false;
            }
        };
        let count = videos.len().min(i32::MAX as usize) as i32;
        let ungrouped_count = videos
            .iter()
            .filter(|video| video.list_id.is_none())
            .count()
            .min(i32::MAX as usize) as i32;
        let list_count = lists.len().min(i32::MAX as usize) as i32;
        self.as_mut().rust_mut().videos = videos;
        self.as_mut().rust_mut().lists = lists;
        self.as_mut().set_in_progress_count(count);
        self.as_mut().set_ungrouped_video_count(ungrouped_count);
        self.as_mut().set_list_count(list_count);
        let revision = self.rust().revision.wrapping_add(1).max(1);
        self.as_mut().set_revision(revision);
        self.as_mut()
            .set_status_message(QString::from("学习库已更新"));
        true
    }

    fn video_title_at(&self, index: i32) -> QString {
        self.rust()
            .videos
            .get(index.max(0) as usize)
            .map(|video| QString::from(&video.title))
            .unwrap_or_else(|| QString::from(""))
    }

    fn video_path_at(&self, index: i32) -> QString {
        self.rust()
            .videos
            .get(index.max(0) as usize)
            .map(|video| QString::from(&video.path))
            .unwrap_or_else(|| QString::from(""))
    }

    fn video_duration_at(&self, index: i32) -> f64 {
        self.rust()
            .videos
            .get(index.max(0) as usize)
            .map(|video| video.duration_secs)
            .unwrap_or(0.0)
    }

    fn create_list(mut self: Pin<&mut Self>, name: &QString) -> bool {
        let result = self
            .as_mut()
            .rust_mut()
            .repository
            .create_video_list(&name.to_string());
        if let Err(err) = result {
            return self.as_mut().report_error("创建列表失败", err);
        }
        self.as_mut().refresh()
    }

    fn delete_list(mut self: Pin<&mut Self>, list_index: i32) -> bool {
        let list_id = match self.rust().lists.get(list_index.max(0) as usize) {
            Some(list) => list.id,
            None => return false,
        };
        let result = self
            .as_mut()
            .rust_mut()
            .repository
            .delete_video_list(list_id);
        if let Err(err) = result {
            return self.as_mut().report_error("删除列表失败", err);
        }
        self.as_mut().refresh()
    }

    fn move_video_to_list(mut self: Pin<&mut Self>, path: &QString, list_index: i32) -> bool {
        let list_id = if list_index < 0 {
            None
        } else {
            match self.rust().lists.get(list_index as usize) {
                Some(list) => Some(list.id),
                None => return false,
            }
        };
        let result = self
            .as_mut()
            .rust_mut()
            .repository
            .move_video_to_list(&path.to_string(), list_id);
        if let Err(err) = result {
            return self.as_mut().report_error("移动视频失败", err);
        }
        self.as_mut().refresh()
    }

    fn remove_video(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        if path.trim().is_empty() {
            return false;
        }
        let result = self
            .as_mut()
            .rust_mut()
            .repository
            .remove_from_learning(&path);
        if let Err(err) = result {
            return self.as_mut().report_error("删除学习视频失败", err);
        }
        self.as_mut().refresh()
    }

    fn list_name_at(&self, list_index: i32) -> QString {
        self.rust()
            .lists
            .get(list_index.max(0) as usize)
            .map(|list| QString::from(&list.name))
            .unwrap_or_else(|| QString::from(""))
    }

    fn list_video_count_at(&self, list_index: i32) -> i32 {
        let list_id = match self.rust().lists.get(list_index.max(0) as usize) {
            Some(list) => list.id,
            None => return 0,
        };
        self.rust()
            .videos
            .iter()
            .filter(|video| video.list_id == Some(list_id))
            .count()
            .min(i32::MAX as usize) as i32
    }

    fn list_video_title_at(&self, list_index: i32, video_index: i32) -> QString {
        self.grouped_video_at(list_index, video_index)
            .map(|video| QString::from(&video.title))
            .unwrap_or_else(|| QString::from(""))
    }

    fn list_video_path_at(&self, list_index: i32, video_index: i32) -> QString {
        self.grouped_video_at(list_index, video_index)
            .map(|video| QString::from(&video.path))
            .unwrap_or_else(|| QString::from(""))
    }

    fn ungrouped_video_title_at(&self, video_index: i32) -> QString {
        self.ungrouped_video_at(video_index)
            .map(|video| QString::from(&video.title))
            .unwrap_or_else(|| QString::from(""))
    }

    fn ungrouped_video_path_at(&self, video_index: i32) -> QString {
        self.ungrouped_video_at(video_index)
            .map(|video| QString::from(&video.path))
            .unwrap_or_else(|| QString::from(""))
    }

    fn grouped_video_at(
        &self,
        list_index: i32,
        video_index: i32,
    ) -> Option<&els_storage::LearningVideo> {
        let list_id = self.rust().lists.get(list_index.max(0) as usize)?.id;
        self.rust()
            .videos
            .iter()
            .filter(|video| video.list_id == Some(list_id))
            .nth(video_index.max(0) as usize)
    }

    fn ungrouped_video_at(&self, video_index: i32) -> Option<&els_storage::LearningVideo> {
        self.rust()
            .videos
            .iter()
            .filter(|video| video.list_id.is_none())
            .nth(video_index.max(0) as usize)
    }

    fn report_error(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}：{error}");
        eprintln!("{message}");
        self.as_mut().set_status_message(QString::from(&message));
        false
    }
}
