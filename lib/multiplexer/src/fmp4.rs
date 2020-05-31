use bytes::{BufMut, Bytes, BytesMut};
use super::{Metadata, Tag};

pub enum Codec {
    MP3,
    AAC,
}

pub struct Flags {
    pub is_leading: u8,
    pub is_depended_on: u8,
    pub depends_on: u8,
    pub has_redundancy: u8,
    pub is_non_sync: u8,
}

pub struct Sample {
    pub duration: u32,
    pub size: u32,
    pub flags: Flags,
    pub cts: u32,
}

pub struct Track {
    pub id: u32,
    pub sequence_number: u32,
    pub samples: Vec<Sample>,
}

/// 包装Box
///
/// 指定Box的类型和下级Box列表，
/// 包装出Box的Bytes数据.
pub fn packet_box(name: &[u8], data: Vec<Bytes>) -> Bytes {
    let mut size = 8u32;
    let mut packet = BytesMut::new();

    // 计算下级Box的长度，
    // 添加到总长度.
    for chunk in &data {
        size += chunk.len() as u32;
    }

    // 写入长度
    // 写入类型
    packet.put_u32(size);
    packet.put_slice(name);

    // 写入全部Box
    for chunk in data {
        packet.extend_from_slice(&chunk);
    }

    packet.freeze()
}

pub fn ftyp() -> Bytes {
    packet_box(b"ftyp", vec![
        Bytes::from([
            0x69, 0x73, 0x6F, 0x6D,  // major_brand: isom
            0x0,  0x0,  0x0,  0x1,   // minor_version: 0x01
            0x69, 0x73, 0x6F, 0x6D,  // isom
            0x61, 0x76, 0x63, 0x31   // avc1
        ].to_vec())
    ])
}

pub fn moov(meta: &Metadata) -> Bytes {
    packet_box(b"moov", vec![mvhd(meta), trak(meta), mvex(meta)])
}

pub fn mvhd(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x00, // version(0) + flags
        0x00, 0x00, 0x00, 0x00, // creation_time
        0x00, 0x00, 0x00, 0x00, // modification_time
    ]);

    packet.put_u32(meta.timescale);
    packet.put_u32(meta.duration);

    packet.put_slice(&[
        0x00, 0x01, 0x00, 0x00, // Preferred rate: 1.0
        0x01, 0x00, 0x00, 0x00, // PreferredVolume(1.0, 2bytes) + reserved(2bytes)
        0x00, 0x00, 0x00, 0x00, // reserved: 4 + 4 bytes
        0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, // ----begin composition matrix----
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00,
        0x00, 0x00, // ----end composition matrix----
        0x00, 0x00, 0x00, 0x00, // ----begin pre_defined 6 * 4 bytes----
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, // ----end pre_defined 6 * 4 bytes----
        0xFF, 0xFF, 0xFF, 0xFF, // next_track_ID
    ]);

    packet_box(b"mvhd", vec![packet.freeze()])
}

pub fn trak(meta: &Metadata) -> Bytes {
    packet_box(b"trak", vec![tkhd(meta), mdia(meta)])
}

pub fn tkhd(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x07, // version(0) + flags
        0x00, 0x00, 0x00, 0x00, // creation_time
        0x00, 0x00, 0x00, 0x00, // modification_time
    ]);

    packet.put_u32(meta.track_id as u32);
    packet.put_u32(0);
    packet.put_u32(meta.duration);

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x00, // reserved: 2 * 4 bytes
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // layer(2bytes) + alternate_group(2bytes)
        0x00, 0x00, 0x00, 0x00, // volume(2bytes) + reserved(2bytes)
        0x00, 0x01, 0x00, 0x00, // ----begin composition matrix----
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00,
        0x00, 0x00, // ----end composition matrix----
    ]);

    packet.put_u16(meta.present_width as u16);
    packet.put_u16(0);
    packet.put_u16(meta.present_height as u16);
    packet.put_u16(0);

    packet_box(b"tkhd", vec![packet.freeze()])
}

pub fn mdia(meta: &Metadata) -> Bytes {
    packet_box(b"meta", vec![mdhd(meta), hdlr(meta), minf(meta)])
}

pub fn mdhd(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x00, // version(0) + flags
        0x00, 0x00, 0x00, 0x00, // creation_time
        0x00, 0x00, 0x00, 0x00, // modification_time
    ]);

    packet.put_u32(meta.timescale);
    packet.put_u32(meta.duration);

    packet.put_slice(&[
        0x55, 0xC4, // language: und (undetermined)
        0x00, 0x00, // pre_defined = 0
    ]);

    packet_box(b"mdhd", vec![packet.freeze()])
}

pub fn hdlr(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    if let Tag::Audio = meta.tag {
        packet.put_slice(&[
            0x00, 0x00, 0x00, 0x00, // version(0) + flags
            0x00, 0x00, 0x00, 0x00, // pre_defined
            0x73, 0x6F, 0x75, 0x6E, // handler_type: 'soun'
            0x00, 0x00, 0x00, 0x00, // reserved: 3 * 4 bytes
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x53, 0x6F, 0x75, 0x6E, 0x64, 0x48,
            0x61, 0x6E, 0x64, 0x6C, 0x65, 0x72, 0x00, // name: SoundHandler
        ]);
    } else {
        packet.put_slice(&[
            0x00, 0x00, 0x00, 0x00, // version(0) + flags
            0x00, 0x00, 0x00, 0x00, // pre_defined
            0x76, 0x69, 0x64, 0x65, // handler_type: 'vide'
            0x00, 0x00, 0x00, 0x00, // reserved: 3 * 4 bytes
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x56, 0x69, 0x64, 0x65, 0x6F, 0x48,
            0x61, 0x6E, 0x64, 0x6C, 0x65, 0x72, 0x00, // name: VideoHandler
        ])
    }

    packet_box(b"hdlr", vec![packet.freeze()])
}

pub fn minf(meta: &Metadata) -> Bytes {
    if let Tag::Audio = meta.tag {
        packet_box(b"minf", vec![smhd(), dinf(), stbl(meta)])
    } else {
        packet_box(b"minf", vec![vmhd(), dinf(), stbl(meta)])
    }
}

pub fn smhd() -> Bytes {
    packet_box(
        b"smhd",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x00, // version(0) + flags
                0x00, 0x00, 0x00, 0x00, // balance(2) + reserved(2)
            ]
            .to_vec(),
        )],
    )
}

pub fn vmhd() -> Bytes {
    packet_box(
        b"vmhd",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x01, // version(0) + flags
                0x00, 0x00, // graphicsmode: 2 bytes
                0x00, 0x00, 0x00, 0x00, // opcolor: 3 * 2 bytes
                0x00, 0x00,
            ]
            .to_vec(),
        )],
    )
}

pub fn dinf() -> Bytes {
    packet_box(b"dinf", vec![dref()])
}

pub fn dref() -> Bytes {
    packet_box(
        b"dref",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x00, // version(0) + flags
                0x00, 0x00, 0x00, 0x01, // entry_count
                0x00, 0x00, 0x00, 0x0C, // entry_size
                0x75, 0x72, 0x6C, 0x20, // type 'url '
                0x00, 0x00, 0x00, 0x01, // version(0) + flags
            ]
            .to_vec(),
        )],
    )
}

pub fn stbl(meta: &Metadata) -> Bytes {
    packet_box(b"stbl", vec![stsd(meta), stts(), stsc(), stsz(), stco()])
}

pub fn stts() -> Bytes {
    packet_box(
        b"stts",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x00, // version(0) + flags
                0x00, 0x00, 0x00, 0x00, // entry_count)
            ]
            .to_vec(),
        )],
    )
}

pub fn stsc() -> Bytes {
    packet_box(
        b"stsc",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x00, // version(0) + flags
                0x00, 0x00, 0x00, 0x00, // entry_count)
            ]
            .to_vec(),
        )],
    )
}

pub fn stsz() -> Bytes {
    packet_box(
        b"stsz",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x00, // version(0) + flags
                0x00, 0x00, 0x00, 0x00, // sample_size
                0x00, 0x00, 0x00, 0x00, // sample_count
            ]
            .to_vec(),
        )],
    )
}

pub fn stco() -> Bytes {
    packet_box(
        b"stco",
        vec![Bytes::from(
            [
                0x00, 0x00, 0x00, 0x00, // version(0) + flags
                0x00, 0x00, 0x00, 0x00, // entry_count)
            ]
            .to_vec(),
        )],
    )
}

pub fn stsd(meta: &Metadata) -> Bytes {
    if let Tag::Audio = meta.tag {
        packet_box(
            b"stsd",
            vec![
                Bytes::from(
                    [
                        0x00, 0x00, 0x00, 0x00, // version(0) + flags
                        0x00, 0x00, 0x00, 0x01, // entry_count
                    ]
                    .to_vec(),
                ),
                mp4a(meta),
            ],
        )
    } else {
        packet_box(
            b"stsd",
            vec![
                Bytes::from(
                    [
                        0x00, 0x00, 0x00, 0x00, // version(0) + flags
                        0x00, 0x00, 0x00, 0x01, // entry_count
                    ]
                    .to_vec(),
                ),
                avc1(meta),
            ],
        )
    }
}

pub fn mp3(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x00, // reserved(4)
        0x00, 0x00, 0x00, 0x01, // reserved(2) + data_reference_index(2)
        0x00, 0x00, 0x00, 0x00, // reserved: 2 * 4 bytes
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]);

    packet.put_u16(meta.channel_count as u16); // channelCount(2)
    packet.put_slice(&[
        0x00, 0x10, // sampleSize(2)
        0x00, 0x00, 0x00, 0x00, // reserved(4)
    ]);

    packet.put_u16(meta.audio_sample_rate as u16); // Audio sample rate
    packet.put_u16(0x00);
    packet_box(b".mp3", vec![packet.freeze()])
}

pub fn mp4a(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x00, // reserved(4)
        0x00, 0x00, 0x00, 0x01, // reserved(2) + data_reference_index(2)
        0x00, 0x00, 0x00, 0x00, // reserved: 2 * 4 bytes
        0x00, 0x00, 0x00, 0x00, 0x00,
    ]);

    packet.put_u16(meta.channel_count as u16); // channelCount(2)
    packet.put_slice(&[
        0x00, 0x10, // sampleSize(2)
        0x00, 0x00, 0x00, 0x00, // reserved(4)
    ]);

    packet.put_u16(meta.audio_sample_rate as u16); // Audio sample rate
    packet.put_u16(0x00);
    packet_box(b"mp4a", vec![packet.freeze(), esds(meta)])
}

pub fn esds(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();
    let size = meta.config.len() as u8;

    packet.put_slice(&[
        0x00,
        0x00,
        0x00,
        0x00, // version 0 + flags
        0x03, // descriptor_type
        0x17 + size, // length3
        0x00,
        0x01, // es_id
        0x00, // stream_priority
        0x04, // descriptor_type
        0x0F + size, // length
        0x40, // codec: mpeg4_audio
        0x15, // stream_type: Audio
        0x00,
        0x00,
        0x00, // buffer_size
        0x00,
        0x00,
        0x00,
        0x00, // maxBitrate
        0x00,
        0x00,
        0x00,
        0x00, // avgBitrate
        0x05, // descriptor_type
    ]);

    packet.put_u8(size);
    packet.put(meta.config.clone());
    packet.put_slice(&[0x06, 0x01, 0x02]); // GASpecificConfig

    packet_box(b"esds", vec![packet.freeze()])
}

pub fn avc1(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x00, // reserved(4)
        0x00, 0x00, 0x00, 0x01, // reserved(2) + data_reference_index(2)
        0x00, 0x00, 0x00, 0x00, // pre_defined(2) + reserved(2)
        0x00, 0x00, 0x00, 0x00, // pre_defined: 3 * 4 bytes
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);

    packet.put_u16(meta.codec_width as u16); // width: 2 bytes
    packet.put_u16(meta.codec_height as u16); // width: 2 bytes

    packet.put_slice(&[
        0x00, 0x48, 0x00, 0x00, // horizresolution: 4 bytes
        0x00, 0x48, 0x00, 0x00, // vertresolution: 4 bytes
        0x00, 0x00, 0x00, 0x00, // reserved: 4 bytes
        0x00, 0x01, // frame_count
        0x0A, // strlen
        0x78, 0x71, 0x71, 0x2F, // compressorname: 32 bytes
        0x66, 0x6C, 0x76, 0x2E, 0x6A, 0x73, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x18, // depth
        0xFF, 0xFF, // pre_defined = -1
    ]);

    packet_box(
        b"avc1",
        vec![
            packet.freeze(),
            packet_box(b"avcC", vec![meta.avcc.clone()]),
        ],
    )
}

pub fn mvex(meta: &Metadata) -> Bytes {
    packet_box(b"mvex", vec![trex(meta)])
}

pub fn trex(meta: &Metadata) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_u32(0x00); // version(0) + flags
    packet.put_u32(meta.track_id as u32); // track_ID
    packet.put_slice(&[
        0x00, 0x00, 0x00, 0x01, // default_sample_description_index
        0x00, 0x00, 0x00, 0x00, // default_sample_duration
        0x00, 0x00, 0x00, 0x00, // default_sample_size
        0x00, 0x01, 0x00, 0x01, // default_sample_flags
    ]);

    packet_box(b"trex", vec![packet.freeze()])
}

pub fn moof(track: &Track, base_media_decode_time: u32) -> Bytes {
    packet_box(
        b"moof",
        vec![
            mfhd(track.sequence_number),
            traf(track, base_media_decode_time),
        ],
    )
}

pub fn mfhd(sequence_number: u32) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_u32(0x00);
    packet.put_u32(sequence_number);

    packet_box(b"mfhd", vec![packet.freeze()])
}

pub fn traf(track: &Track, base_media_decode_time: u32) -> Bytes {
    let sdtp_value = sdtp(track);
    packet_box(
        b"traf",
        vec![
            tfhd(track),
            tfdt(base_media_decode_time),
            trun(track, (sdtp_value.len() + 72) as u32),
            sdtp_value,
        ],
    )
}

pub fn tfhd(track: &Track) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_u32(0x00);
    packet.put_u32(track.id);

    packet_box(b"tfhd", vec![packet.freeze()])
}

pub fn tfdt(base_media_decode_time: u32) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_u32(0x00);
    packet.put_u32(base_media_decode_time);

    packet_box(b"tfdt", vec![packet.freeze()])
}

pub fn trun(track: &Track, offset: u32) -> Bytes {
    let size = track.samples.len() as u32;
    let data_size = 12 + 16 * size;
    let mut packet = BytesMut::new();

    packet.put_slice(&[0x00, 0x00, 0x0F, 0x01]);
    packet.put_u32(size);
    packet.put_u32(offset + 8 + data_size);

    for sample in &track.samples {
        packet.put_u32(sample.duration);
        packet.put_u32(sample.size);
        packet.put_slice(&[
            (sample.flags.is_leading << 2) | sample.flags.depends_on, // sample_flags
            (sample.flags.is_depended_on << 6)
                | (sample.flags.has_redundancy << 4)
                | sample.flags.is_non_sync,
            0x00,
            0x00, // sample_degradation_priority
        ]);

        packet.put_u32(sample.cts);
    }

    packet_box(b"trun", vec![packet.freeze()])
}

pub fn sdtp(track: &Track) -> Bytes {
    let mut packet = BytesMut::new();

    packet.put_u32(0x00);

    for sample in &track.samples {
        packet.put_u32_le(
            ((sample.flags.is_leading << 6)
                | (sample.flags.depends_on << 4)
                | (sample.flags.is_depended_on << 2)
                | (sample.flags.has_redundancy)) as u32,
        );
    }

    packet_box(b"sdtp", vec![packet.freeze()])
}

pub fn mdat(data: Bytes) -> Bytes {
    packet_box(b"mdat", vec![data])
}
