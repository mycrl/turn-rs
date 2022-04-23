#include "libavcodec/avcodec.h"
#include "libavutil/imgutils.h"
#include "ffi.h"

__declspec(dllexport) const AVCodec* create_encoder(const char* name)
{
	return avcodec_find_encoder_by_name(name);
}

__declspec(dllexport) struct Encoder* open_encoder(struct EncoderOptions* options)
{
	int ret;
	struct Encoder* encoder = malloc(sizeof(struct Encoder));
	if (encoder == NULL) {
		return NULL;
	}

	encoder->pts = 0;
	encoder->options = options;
	encoder->ctx = avcodec_alloc_context3(encoder->options->codec);
	if (encoder->ctx == NULL) {
		return NULL;
	}
    
    if (encoder->options->codec->id != AV_CODEC_ID_H264) {
        return NULL;
    }

	encoder->ctx->width = options->width;
	encoder->ctx->height = options->height;
	encoder->ctx->time_base = av_make_q(1, options->frame_rate);
	encoder->ctx->pkt_timebase = av_make_q(1, options->frame_rate);
	encoder->ctx->framerate = av_make_q(options->frame_rate, 1);
	encoder->ctx->gop_size = options->frame_rate * 2;
	encoder->ctx->max_b_frames = 3;
	encoder->ctx->pix_fmt = options->format;
	encoder->ctx->bit_rate = options->bit_rate;
    
	ret = avcodec_open2(encoder->ctx, encoder->options->codec, NULL);
	if (ret != 0) {
		return NULL;
	}

	encoder->packet = av_packet_alloc();
	if (encoder->packet == NULL) {
		return NULL;
	}

	encoder->frame = av_frame_alloc();
	if (encoder->frame == NULL) {
		return NULL;
	}

	encoder->frame->format = encoder->ctx->pix_fmt;
	encoder->frame->width = encoder->ctx->width;
	encoder->frame->height = encoder->ctx->height;

	ret = av_frame_get_buffer(encoder->frame, 32);
	if (ret < 0) {
		return NULL;
	}

	return encoder;
}

__declspec(dllexport) int encoder_get_buffer_size(struct Encoder* encoder)
{
	int frame_bytes = av_image_get_buffer_size(
		encoder->frame->format,
		encoder->frame->width,
		encoder->frame->height,
		1);
	return frame_bytes;
} 

__declspec(dllexport) enum CodecStatus encoder_write_frame(struct Encoder* encoder, const uint8_t* buf, int frame_bytes)
{
    int ret;
    ret = av_frame_make_writable(encoder->frame);
    if (ret != 0) {
        return RR_ERROR;
    }
    
	int need_size = av_image_fill_arrays(
		encoder->frame->data,
		encoder->frame->linesize,
		buf,
		encoder->frame->format,
		encoder->frame->width,
		encoder->frame->height,
		1);
	if (need_size != frame_bytes) {
		return RR_ERROR;
	}

	encoder->pts += 1;
    encoder->frame->pts = encoder->pts;
    
	ret = avcodec_send_frame(encoder->ctx, encoder->frame);
	return av_err_to_status(ret);
}

__declspec(dllexport) enum CodecStatus encoder_receiver(struct Encoder* encoder)
{
	int ret = avcodec_receive_packet(encoder->ctx, encoder->packet);
	encoder->packet->stream_index = 0;
	return av_err_to_status(ret);
}

__declspec(dllexport) struct Chunk encoder_get_pkt_chunk(struct Encoder* encoder)
{
	struct Chunk chunk;
	chunk.data = encoder->packet->data;
	chunk.len = encoder->packet->size;
	return chunk;
}

__declspec(dllexport) void encoder_clean(struct Encoder* encoder)
{
	av_packet_unref(encoder->packet);
}

__declspec(dllexport) void encoder_free(struct Encoder* encoder) 
{
    avcodec_send_frame(encoder->ctx, NULL);
	av_frame_free(&encoder->frame);
	av_packet_free(&encoder->packet);
	avcodec_free_context(&encoder->ctx);
	free(encoder);
}

enum CodecStatus av_err_to_status(int ret) 
{
    if (ret == 0) {
		return RR_Ready;
	}
    
	if (ret == AVERROR(EAGAIN)) {
		return RR_Wait;
	}
    
	if (ret == AVERROR_EOF) {
		return RR_Eof;
	}

	return RR_ERROR;
}
