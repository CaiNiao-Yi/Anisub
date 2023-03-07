use std::path::Path;

use ffmpeg_next::{
    codec, encoder,
    format::{input, output, stream::Disposition},
    media::Type,
    Packet, Stream,
};

#[derive(Debug)]
pub struct Video {
    name: String,
    output_name: String,
    video_path: String,
    subtitle_path: String,
    language: String,
}

impl Video {
    pub fn new<P: AsRef<Path>>(video_path: &P, subtitle_path: &P) -> Option<Self> {
        let video_path = video_path.as_ref();
        let subtitle_path = subtitle_path.as_ref();
        if video_path.is_file() && video_path.exists() {
            let name = video_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            let mut name_without_extend = "";
            if let Some(i) = name.rfind(".") {
                name_without_extend = &name[..i];
            }

            let subtitle_name = subtitle_path.to_str().unwrap().to_string();
            let mut language = "None";
            if subtitle_name.contains("zh")
                || subtitle_name.contains("ch")
                || subtitle_name.contains("sc")
            {
                language = "zh";
            } else if subtitle_name.contains("en") {
                language = "en";
            } else if subtitle_name.contains("jp") {
                language = "jp";
            }
            let output_name = format!("{0}[{1}].mkv", name_without_extend, language);
            Some(Self {
                subtitle_path: subtitle_path.to_str().unwrap().to_string(),
                name: video_path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                video_path: video_path.to_str().unwrap().to_string(),
                output_name,
                language: language.to_owned(),
            })
        } else {
            None
        }
    }
    pub fn mux(self) {
        let mut in_video_ctx = input(&self.video_path).unwrap();
        let mut out_video_ctx = output(&self.output_name).unwrap();
        // 设置章节
        for c in in_video_ctx.chapters() {
            let title = match c.metadata().get("title") {
                Some(t) => String::from(t),
                None => String::from("No Title"),
            };
            out_video_ctx
                .add_chapter(c.id(), c.time_base(), c.start(), c.end(), title)
                .unwrap();
        }
        // 筛选输入流
        let mut output_str_map =
            vec![0 as i32; in_video_ctx.nb_streams() as usize + self.subtitle_path.len()];
        let mut output_index = 0;
        for (i, s) in in_video_ctx.streams().enumerate() {
            if s.parameters().medium() != Type::Video
                && s.parameters().medium() != Type::Audio
                && s.parameters().medium() != Type::Subtitle
                && s.parameters().medium() != Type::Attachment
            {
                output_str_map[i] = -1;
                continue;
            }
            let mut out_stream = out_video_ctx
                .add_stream(encoder::find(codec::Id::None))
                .unwrap();
            out_stream.set_parameters(s.parameters());
            out_stream.set_metadata(s.metadata().to_owned());
            output_str_map[i] = output_index;
            output_index += 1;
        }
        // 处理字幕流
        let subtitle_shift = output_index;
        let in_subtitle_ctx = input(&self.subtitle_path).unwrap();
        let mut out_stream = out_video_ctx
            .add_stream(encoder::find(codec::Id::None))
            .unwrap();
        out_stream.set_parameters(in_subtitle_ctx.stream(0).unwrap().parameters());
        let mut md = in_subtitle_ctx.stream(0).unwrap().metadata().to_owned();
        md.set("language", &self.language);
        out_stream.set_metadata(md);
        out_stream.disposition().set(Disposition::DEFAULT, true);
        output_str_map[subtitle_shift as usize] = output_index;
        // 处理视频包
        out_video_ctx.set_metadata(in_video_ctx.metadata().to_owned());
        out_video_ctx.write_header().unwrap();
        let video_packets = in_video_ctx
            .packets()
            .filter_map(|(s, mut p)| {
                if output_str_map[s.index()] < 0 {
                    None
                } else {
                    let index = output_str_map[s.index()];
                    let os = out_video_ctx.stream(index as _).unwrap();
                    p.set_stream(index as _);
                    p.rescale_ts(s.time_base(), os.time_base());
                    p.set_position(-1);
                    Some((s, p))
                }
            })
            .collect::<Vec<_>>();
        let mut output_packets = video_packets;
        // 处理字幕包
        let mut in_subtitle_ctx = input(&self.subtitle_path).unwrap();
        let mut subtitle_packets = in_subtitle_ctx
            .packets()
            .filter_map(|(s, mut p)| {
                if output_str_map[s.index()] < 0 {
                    None
                } else {
                    let index = output_str_map[subtitle_shift as usize];
                    let os = out_video_ctx.stream(index as _).unwrap();
                    p.set_stream(index as _);
                    p.rescale_ts(s.time_base(), os.time_base());
                    p.set_position(-1);
                    Some((s, p))
                }
            })
            .collect::<Vec<_>>();
        output_packets.append(&mut subtitle_packets);
        output_packets.sort_by(Self::comp);
        for (_, p) in output_packets {
            p.write_interleaved(&mut out_video_ctx).unwrap();
        }
        out_video_ctx.write_trailer().unwrap();
    }
    fn comp(a: &(Stream, Packet), b: &(Stream, Packet)) -> std::cmp::Ordering {
        a.1.dts()
            .unwrap_or_default()
            .cmp(&b.1.dts().unwrap_or_default())
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}
