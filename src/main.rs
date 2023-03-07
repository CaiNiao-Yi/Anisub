use std::process::exit;

use log::warn;
use tracing_subscriber::{
    self, fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};
use video::Video;
use walkdir::WalkDir;

mod video;

fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();
    if std::env::args().len() < 3 {
        warn!("参数不足");
        println!("使用方法\nanisub [path] [video extend] [subtitle extend]");
        exit(-1);
    }
    let binding = String::from(std::env::args().nth(1).unwrap());
    let target_path = binding.as_str();
    let binding = String::from(std::env::args().nth(2).unwrap());
    let video_extend = binding.as_str();
    let binding = String::from(std::env::args().nth(3).unwrap());
    let subtitle_extend = binding.as_str();

    let mut video_list: Vec<Video> = vec![];
    let video_vec = WalkDir::new(target_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|d| d.ok())
        .filter(|d| {
            d.file_type().is_file() && d.file_name().to_str().unwrap().ends_with(video_extend)
        })
        .collect::<Vec<_>>();
    let subtitle_vec = WalkDir::new(target_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|d| d.ok())
        .filter(|d| {
            d.file_type().is_file() && d.file_name().to_str().unwrap().ends_with(subtitle_extend)
        })
        .collect::<Vec<_>>();

    for path in video_vec {
        let last_index = match path.file_name().to_str() {
            Some(s) => match s.to_string().rfind(".") {
                Some(i) => i,
                None => {
                    warn!("无法查找文件拓展名");
                    s.len() - video_extend.len() + 1
                }
            },

            None => {
                warn!("无法转换文件名");
                continue;
            }
        };

        let file_name = match path.file_name().to_str() {
            Some(s) => s.to_string().clone()[..last_index].to_string(),
            None => {
                warn!("无法转换文件名");
                continue;
            }
        };

        for subtitle_path in &subtitle_vec {
            if subtitle_path
                .file_name()
                .to_str()
                .unwrap()
                .starts_with(&file_name)
            {
                let video = Video::new(&path.path(), &subtitle_path.path()).unwrap();
                video_list.push(video);
                break;
            }
        }
    }
    ffmpeg_next::init().unwrap();
    ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Error);
    let leng = video_list.len() as u64;
    for (i, v) in video_list.into_iter().enumerate() {
        println!("正在处理:{0} {1:>}", v.name(), format!("{}/{}", i, leng));
        v.mux();
    }
}
