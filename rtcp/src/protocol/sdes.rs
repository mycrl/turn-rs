
/// SDES: Source Description RTCP Packet
/// 
/// ```text
///        0                   1                   2                   3
///        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// header |V=2|P|    SC   |  PT=SDES=202  |             length            |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// chunk  |                          SSRC/CSRC_1                          |
///   1    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                           SDES items                          |
///        |                              ...                              |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// chunk  |                          SSRC/CSRC_2                          |
///   2    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        |                           SDES items                          |
///        |                              ...                              |
///        +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// ```
/// 
/// The SDES packet is a three-level structure composed of a header and
/// zero or more chunks, each of which is composed of items describing
/// the source identified in that chunk.  The items are described
/// individually in subsequent sections.
/// 
/// version (V), padding (P), length:
/// As described for the SR packet.
/// 
/// packet type (PT): 8 bits
/// Contains the constant 202 to identify this as an RTCP SDES packet.
/// 
/// source count (SC): 5 bits
/// The number of SSRC/CSRC chunks contained in this SDES packet.  A
/// value of zero is valid but useless.
/// 
/// Each chunk consists of an SSRC/CSRC identifier followed by a list of
/// zero or more items, which carry information about the SSRC/CSRC.
/// Each chunk starts on a 32-bit boundary.  Each item consists of an 8-
/// bit type field, an 8-bit octet count describing the length of the
/// text (thus, not including this two-octet header), and the text
/// itself.  Note that the text can be no longer than 255 octets, but
/// this is consistent with the need to limit RTCP bandwidth consumption.
pub struct Sdes {
    
}