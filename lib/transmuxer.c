#include <libavformat/avformat.h>

int transmuxer() {
    AVOutputFormat *ofmt = NULL;
    AVFormatContext *ifmt_ctx_v = NULL, *ifmt_ctx_a = NULL,*ofmt_ctx = NULL;
	AVPacket pkt;
	int ret, i;
	int videoindex_v = -1；
    int videoindex_out = -1;
	int audioindex_a = -1;
    int audioindex_out = -1;
	int frame_index = 0;
	int64_t cur_pts_v = 0,
    int cur_pts_a = 0;

    av_register_all();
    
}