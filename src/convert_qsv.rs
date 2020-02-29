use std::cmp::min;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;

use chrono::Local;

use crate::error;
use crate::flv_format::{KeyFrame, MetaData};

const TAG_TAIL_SIZE_LEN: usize = 4;

const DEFAULT_COPY_BUFF_SIZE: usize = 8388608; // 默认的TAG拷贝缓冲区大小设为8MB

pub enum FlvTagType {
    Audio = 0x08,
    Video = 0x09,
    Script = 0x12,
}

impl FlvTagType {
    pub const TAG_TYPE_AUDIO: u8 = 0x08;
    pub const TAG_TYPE_VIDEO: u8 = 0x09;
    pub const TAG_TYPE_SCRIPT: u8 = 0x12;

    fn from_byte(val: u8) -> Option<Self> {
        match val {
            Self::TAG_TYPE_AUDIO => Some(Self::Audio),
            Self::TAG_TYPE_VIDEO => Some(Self::Video),
            Self::TAG_TYPE_SCRIPT => Some(Self::Script),
            _ => None,
        }
    }

    // fn is_tag_type_valid(val: u8) -> bool {
    //     match val {
    //         Self::TAG_TYPE_AUDIO | Self::TAG_TYPE_VIDEO | Self::TAG_TYPE_SCRIPT => true,
    //         _ => false,
    //     }
    // }
}

// 这里的Tag所包含的数据：
// TagHeader + TagData + SizeOf(TagHeader+TagData)
pub struct FlvTagBlock {
    pub tag_type: FlvTagType,
    pub offset: u64,
    pub size: u64,
}

pub type FlvTagBlocks = Vec<FlvTagBlock>;

pub fn validate_qsv_format(qsv: &mut File) -> error::Result<()> {
    const QIYI_TAG: &[u8] = b"QIYI VIDEO";

    let mut qsv_tag: [u8; QIYI_TAG.len()] = [0; QIYI_TAG.len()];
    let mut qsv_ver: [u8; 4] = [0; 4];
    qsv.seek(SeekFrom::Start(0))?;
    // 不必担心文件读取的字节数小于buffer大小的情况
    qsv.read(&mut qsv_tag)?;
    qsv.read(&mut qsv_ver)?;

    if i32::from_le_bytes(qsv_ver) != 2 {
        Err(error::Error::from(error::ErrorKind::IncorrectQsvVersion))
    } else if qsv_tag != QIYI_TAG {
        Err(error::Error::from(error::ErrorKind::IncorrectQsvFormat))
    } else {
        Ok(())
    }
}

pub fn seek_qsv_to_start(qsv: &mut File) -> std::io::Result<bool> {
    qsv.seek(SeekFrom::Start(0x4A))?;
    let mut buffer = [0u8; 12];
    qsv.read_exact(&mut buffer)?;
    let mut buffer_offset = [0u8; 8];
    let mut buffer_size = [0u8; 4];
    buffer_offset.clone_from_slice(&buffer[0..8]);
    buffer_size.clone_from_slice(&buffer[8..12]);

    // 视频信息的偏移（起始位置）
    let offset: u64 = u64::from_le_bytes(buffer_offset);
    // 视频信息的长度
    let size: u64 = u32::from_le_bytes(buffer_size) as u64;
    // dbg!(size, offset);

    if size + offset > qsv.metadata()?.len() {
        Ok(false)
    } else {
        qsv.seek(SeekFrom::Start(offset + size))?;
        Ok(true)
    }
}

pub fn skip_qsv_metadata(qsv: &mut File) -> std::io::Result<()> {
    qsv.seek(SeekFrom::Current(0x0D))?;
    let mut len: u32 = 0;
    let mut buf = [0u8; 4]; // 共用的缓冲区

    loop {
        qsv.read_exact(&mut buf[0..1])?; // 读取1个字节
        len += 1;
        if buf[0] == 0x09 {
            qsv.read_exact(&mut buf)?;
            if u32::from_be_bytes(buf) == len {
                break;
            } else {
                qsv.seek(SeekFrom::Current(-4))?;
            }
        }
    }

    Ok(())
}

pub fn tag_blocks_from_qsv(qsv: &mut File) -> std::io::Result<FlvTagBlocks> {
    #[inline]
    fn can_read_a_tag_header(qsv: &mut File) -> std::io::Result<bool> {
        Ok(qsv.seek(SeekFrom::Current(0))? + 11 < qsv.metadata()?.len())
    }

    #[inline]
    fn is_qsv_at_tag_start(qsv: &mut File) -> std::io::Result<bool> {
        let mut buf = [0u8; 11];
        qsv.read_exact(&mut buf)?;
        qsv.seek(SeekFrom::Current(-11))?;

        if let Some(_) = FlvTagType::from_byte(buf[0]) {
            Ok(&buf[8..11] == [0u8; 3])
        } else {
            Ok(false)
        }
    }

    #[inline]
    fn move_qsv_to_next_tag(qsv: &mut File) -> std::io::Result<()> {
        let mut buf = [0u8; 4];
        qsv.read_exact(&mut buf)?;
        let data_size = 0x00FFFFFFu32 & u32::from_be_bytes(buf);
        qsv.seek(SeekFrom::Current(data_size as i64 + 11))?;
        Ok(())
    }

    #[inline]
    fn read_a_tag_from_here(qsv: &mut File) -> std::io::Result<Option<FlvTagBlock>> {
        let mut buf = [0u8; 4];
        qsv.read_exact(&mut buf)?;
        qsv.seek(SeekFrom::Current(-4))?;

        if let Some(tag_type) = FlvTagType::from_byte(buf[0]) {
            let offset = qsv.seek(SeekFrom::Current(0))?;
            let size = (0x00FFFFFFu32 & u32::from_be_bytes(buf)) as u64 + 11; // 11=4+3+1+3
            qsv.seek(SeekFrom::Current(size as i64 + 4))?; // (11+tag_body_size)+4
            Ok(Some(FlvTagBlock {
                tag_type,
                offset,
                size,
            }))
        } else {
            Ok(None)
        }
    }

    let mut blocks = FlvTagBlocks::new();

    seek_qsv_to_start(qsv)?;
    skip_qsv_metadata(qsv)?;
    while can_read_a_tag_header(qsv)? {
        if is_qsv_at_tag_start(qsv)? {
            match read_a_tag_from_here(qsv)? {
                Some(block) => blocks.push(block),
                None => (), // TODO None可能是由于QSV文件格式不正确
            }
        } else {
            skip_qsv_metadata(qsv)?;
            move_qsv_to_next_tag(qsv)?;
            move_qsv_to_next_tag(qsv)?;
        }
    }

    Ok(blocks)
}

pub fn meta_data_from_tag_blocks(qsv: &mut File, tags: &FlvTagBlocks) -> error::Result<MetaData> {
    #[inline]
    fn parse_video_tag(qsv: &mut File, tag: &FlvTagBlock) -> error::Result<(bool, u8)> {
        const TAG_VIDEO_MIN_SIZE: u64 = 12u64;
        if tag.size < TAG_VIDEO_MIN_SIZE {
            // 该TAG的大小不能少于本函数所需的读取字节数（本函数需要在单个TAG块中至少读取12字节）
            return Err(error::Error::from(error::ErrorKind::IncorrectQsvFormat));
        }
        qsv.seek(SeekFrom::Start(tag.offset))?;
        let mut buf = [0u8; 1];
        qsv.read_exact(&mut buf)?;
        if buf != [FlvTagType::TAG_TYPE_VIDEO] {
            // 不是一个视频Tag
            return Err(error::Error::from(error::ErrorKind::IncorrectQsvFormat));
        }
        qsv.seek(SeekFrom::Current(10))?;
        qsv.read_exact(&mut buf)?;
        qsv.seek(SeekFrom::Current(-12))?;

        // 该视频Tag是否为一个关键帧
        let is_key_frame = buf[0] & 0x11u8 == 0x11u8;
        let video_codec_id = buf[0] & 0x0Fu8;
        Ok((is_key_frame, video_codec_id))
    }

    #[inline]
    fn parse_audio_tag(qsv: &mut File, tag: &FlvTagBlock) -> error::Result<(u8, u8, u8, bool)> {
        const TAG_AUDIO_MIN_SIZE: u64 = 12u64;
        if tag.size < TAG_AUDIO_MIN_SIZE {
            // 该TAG的大小不能少于本函数所需的读取字节数（本函数需要在单个TAG块中至少读取12字节）
            return Err(error::Error::from(error::ErrorKind::IncorrectQsvFormat));
        }
        qsv.seek(SeekFrom::Start(tag.offset))?;
        let mut buf = [0u8; 1];
        qsv.read_exact(&mut buf)?;
        if buf != [FlvTagType::TAG_TYPE_AUDIO] {
            // 不是一个音频Tag
            return Err(error::Error::from(error::ErrorKind::IncorrectQsvFormat));
        }
        qsv.seek(SeekFrom::Current(10))?;
        qsv.read_exact(&mut buf)?;
        qsv.seek(SeekFrom::Current(-12))?;

        let sound_format = buf[0] >> 4;
        let sound_rate = (buf[0] >> 2) & 0x03;
        let sound_size = (buf[0] >> 1) & 0x01;
        let sound_stereo = buf[0] & 0x01;
        Ok((sound_format, sound_rate, sound_size, sound_stereo == 0x01))
    }

    #[inline]
    fn get_time_stamp_from_tag(qsv: &mut File, tag: &FlvTagBlock) -> error::Result<i32> {
        const TAG_MIN_SIZE: u64 = 8u64;
        if tag.size < TAG_MIN_SIZE {
            // 该TAG的大小不能少于本函数所需的读取字节数（本函数需要在单个TAG块中至少读取8字节）
            return Err(error::Error::from(error::ErrorKind::IncorrectQsvFormat));
        }
        qsv.seek(SeekFrom::Start(tag.offset))?;
        let mut buf = [0u8; 4];
        qsv.seek(SeekFrom::Current(4))?;
        qsv.read_exact(&mut buf)?;
        qsv.seek(SeekFrom::Current(-8))?;
        let timestamp = i32::from_be_bytes([buf[3], buf[0], buf[1], buf[2]]);

        Ok(timestamp)
    }

    let mut meta = MetaData {
        duration: 0.0,
        width: 0,
        height: 0,
        video_data_rate: 0.0,
        frame_rate: 0.0,
        video_codec_id: 0,
        audio_sample_rate: 0,
        audio_sample_size: 0,
        audio_stereo: true,
        audio_codec_id: 0,
        timestamp_last: 0.0,
        timestamp_last_key_frame: 0.0,
        audio_delay: 0.0,
        can_seek_to_end: true,
        date_creation: Local::now().format("%Y-%m-%d;").to_string(),
        meta_data_creator: String::from("qsv2flv"),
        key_frames: vec![],
        samples: [0, 0, 0, 0, 0, 0],
    };

    if tags.len() > 0 {
        meta.duration = get_time_stamp_from_tag(qsv, tags.last().unwrap())? as f64 / 1000f64;
        meta.timestamp_last = meta.duration;

        for tag in tags {
            if let FlvTagType::Video = &tag.tag_type {
                let (is_key_frame, video_codec_id) = parse_video_tag(qsv, tag)?;
                if is_key_frame {
                    meta.video_codec_id = video_codec_id;
                    break;
                }
            }
        }
        for tag in tags {
            if let FlvTagType::Audio = &tag.tag_type {
                let (audio_codec_id, sample_rate, sample_size, audio_stereo) =
                    parse_audio_tag(qsv, tag)?;
                meta.audio_codec_id = audio_codec_id;
                meta.audio_sample_rate = sample_rate;
                meta.audio_sample_size = sample_size;
                meta.audio_stereo = audio_stereo;
                break;
            }
        }

        const SAMPLES_NUM: usize = 6;
        let mut samples: Vec<i32> = vec![];
        for tag in tags {
            if samples.len() < SAMPLES_NUM {
                if let FlvTagType::Video = &tag.tag_type {
                    let timestamp = get_time_stamp_from_tag(qsv, tag)?;
                    if timestamp >= 1 {
                        samples.push(timestamp);
                    }
                }
            } else {
                break;
            }
        }

        if samples.len() != SAMPLES_NUM {
            return Err(error::Error::from(
                error::ErrorKind::MediaDurationIsTooShort,
            ));
        }

        // 帧率（单位：FPS）
        meta.frame_rate = 9000.0f64
            / (samples[5] + samples[4] + samples[3] - samples[2] - samples[1] - samples[0]) as f64;
        const BREAKING: i32 = 2000; // 两帧之间的Breaking时间（单位：ms）
        let mut last_timestamp = -BREAKING;
        let mut file_pos = 0u64;
        for tag in tags.iter() {
            if let FlvTagType::Video = &tag.tag_type {
                if parse_video_tag(qsv, tag)?.0 {
                    let timestamp = get_time_stamp_from_tag(qsv, tag)?;
                    if timestamp - last_timestamp >= BREAKING {
                        meta.key_frames.push(KeyFrame {
                            file_pos,
                            time_pos: timestamp as f64 / 1000f64,
                        });
                        last_timestamp = timestamp;
                    }
                }
            }
            file_pos += tag.size + TAG_TAIL_SIZE_LEN as u64;
        }
        match meta.key_frames.last() {
            Some(last_key_frame) => meta.timestamp_last_key_frame = last_key_frame.time_pos,
            _ => {}
        }
    } else {
        return Err(error::Error::from(error::ErrorKind::QsvTagsIsEmpty));
    }

    return Ok(meta);
}

pub fn write_from_qsv_to_flv(
    qsv: &mut File,
    tags: &FlvTagBlocks,
    flv: &mut File,
    meta: &MetaData,
) -> std::io::Result<()> {
    trait SeekPlus {
        fn tell(&mut self) -> std::io::Result<u64>;
    }
    impl SeekPlus for File {
        fn tell(&mut self) -> std::io::Result<u64> {
            self.seek(SeekFrom::Current(0))
        }
    }
    flv.seek(SeekFrom::Start(0))?;
    flv.write_all(&MetaData::META_1)?;

    flv.seek(SeekFrom::Start(MetaData::POS_DURATION))?;
    flv.write_all(&meta.duration.to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_VIDEO_DATA_RATE))?;
    flv.write_all(&meta.video_data_rate.to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_FRAME_RATE))?;
    flv.write_all(&meta.frame_rate.to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_VIDEO_CODEC_ID))?;
    flv.write_all(&(meta.video_codec_id as f64).to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_AUDIO_SAMPLE_RATE))?;
    flv.write_all(&(meta.audio_sample_rate as f64).to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_AUDIO_SAMPLE_SIZE))?;
    flv.write_all(&(meta.audio_sample_size as f64).to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_AUDIO_STEREO))?;
    flv.write_all(&[meta.audio_stereo as u8])?;

    flv.seek(SeekFrom::Start(MetaData::POS_AUDIO_CODEC_ID))?;
    flv.write_all(&(meta.audio_codec_id as f64).to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_TIMESTAMP_LAST))?;
    flv.write_all(&meta.timestamp_last.to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_TIMESTAMP_LAST_KEY_FRAME))?;
    flv.write_all(&meta.timestamp_last_key_frame.to_be_bytes())?;

    flv.seek(SeekFrom::Start(MetaData::POS_CAN_SEEK_TO_END))?;
    flv.write_all(&[meta.can_seek_to_end as u8])?;

    flv.seek(SeekFrom::Start(MetaData::POS_DATE_CREATION))?;
    let date_creation_bytes = meta.date_creation.as_bytes();
    flv.write_all(&date_creation_bytes[..min(11, date_creation_bytes.len())])?;

    flv.seek(SeekFrom::Start(MetaData::POS_META_DATA_CREATOR))?;
    flv.write_all(&(meta.meta_data_creator.len() as u16).to_be_bytes())?;
    flv.write_all(&meta.meta_data_creator.as_bytes())?;

    flv.seek(SeekFrom::End(0))?;
    flv.write_all(&MetaData::META_2)?;

    let key_frames = &meta.key_frames;
    let key_frames_len = key_frames.len();
    let header_size = key_frames_len * 18 + meta.meta_data_creator.len() + 466;
    flv.write_all(&(key_frames_len as u32).to_be_bytes())?;
    for key_frame in key_frames.iter() {
        flv.write_all(&0u8.to_be_bytes())?;
        let file_pos = (header_size as u64 + key_frame.file_pos) as f64;
        flv.write_all(&file_pos.to_be_bytes())?;
    }
    flv.write_all(&MetaData::META_3)?;
    flv.write_all(&(key_frames_len as u32).to_be_bytes())?;
    for key_frame in key_frames.iter() {
        flv.write_all(&0u8.to_be_bytes())?;
        flv.write_all(&(key_frame.time_pos as f64).to_be_bytes())?;
    }

    let seek_pos = flv.tell()? - 18;
    flv.seek(SeekFrom::Start(MetaData::POS_META_DATA_SIZE))?;
    flv.write_all(&(seek_pos as u32).to_be_bytes()[1..4])?;

    flv.seek(SeekFrom::End(0))?;
    flv.write_all(&MetaData::META_4)?;

    let seek_pos = flv.tell()? - 13;
    flv.write_all(&(seek_pos as u32).to_be_bytes())?;

    flv.seek(SeekFrom::End(0))?;
    let mut buf = vec![0u8; DEFAULT_COPY_BUFF_SIZE];
    for tag in tags {
        buf.resize(tag.size as usize + TAG_TAIL_SIZE_LEN, 0);
        qsv.seek(SeekFrom::Start(tag.offset))?;
        qsv.read_exact(buf.as_mut())?;
        flv.write_all(buf.as_ref())?;
    }

    flv.seek(SeekFrom::Start(MetaData::POS_FILE_SIZE))?;
    //    meta.file_size = &flv.metadata()?.len();
    flv.write_all(&(flv.metadata()?.len() as f64).to_be_bytes()[..4])?;

    Ok(())
}
