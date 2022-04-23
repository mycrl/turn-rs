#pragma once

#ifdef FFI_EXPORTS
#define FFI_API __declspec(dllexport)
#else
#define FFI_API __declspec(dllimport)
#endif

#include "libavcodec/avcodec.h"

FFI_API enum CodecStatus {
	RR_ERROR,
	RR_Ready,
	RR_Wait,
	RR_Eof
};

FFI_API struct Chunk {
	uint8_t* data;
	int len;
};

FFI_API struct Encoder {
	struct EncoderOptions* options;
	AVCodecContext* ctx;
	AVPacket* packet;
	AVFrame* frame;
	int64_t pts;
};

FFI_API struct EncoderOptions {
	const AVCodec* codec;
	int32_t width;
	int32_t height;
	int64_t bit_rate;
	int32_t frame_rate;
	enum AVPixelFormat format;
};

FFI_API const AVCodec* create_encoder(const char* name);
FFI_API struct Encoder* open_encoder(struct EncoderOptions* options);
FFI_API int32_t encoder_get_buffer_size(struct Encoder* encoder);
FFI_API void encoder_free(struct Encoder* encoder);
FFI_API enum CodecStatus encoder_write_frame(struct Encoder* encoder, const uint8_t* buf, int frame_bytes);
FFI_API enum CodecStatus encoder_receiver(struct Encoder* encoder);
FFI_API struct Chunk encoder_get_pkt_chunk(struct Encoder* encoder);
FFI_API void encoder_clean(struct Encoder* encoder);
