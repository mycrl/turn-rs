/// ### Secure RTP
///
/// RTP is the Real-time Transport Protocol 
/// [RFC3550](https://tools.ietf.org/html/rfc3550).  
/// We define SRTP as a profile of RTP.  This profile is an extension 
/// to the RTP Audio/Video Profile [RFC3551](https://tools.ietf.org/html/rfc3551).  
/// Except where explicitly noted, all aspects of that profile apply, 
/// with the addition of the SRTP security features.  Conceptually, 
/// we consider SRTP to be a "bump in the stack" implementation which 
/// resides between the RTP application and the transport layer.  
/// SRTP intercepts RTP packets and then forwards an equivalent SRTP 
/// packet on the sending side, and intercepts SRTP packets and passes 
/// an equivalent RTP packet up the stack on the receiving side.
/// 
/// Secure RTCP (SRTCP) provides the same security services to RTCP as
/// SRTP does to RTP.  SRTCP message authentication is MANDATORY and
/// thereby protects the RTCP fields to keep track of membership, provide
/// feedback to RTP senders, or maintain packet sequence counters.  SRTCP
/// is described in [Section 3.4](https://tools.ietf.org/html/rfc3711#section-3.4).
///
/// ```bash
///     0                   1                   2                   3
///   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+<+
///   |V=2|P|X|  CC   |M|     PT      |       sequence number         | |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+ |
///   |                           timestamp                           | |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+ |
///   |           synchronization source (SSRC) identifier            | |
///   +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+ |
///   |            contributing source (CSRC) identifiers             | |
///   |                               ....                            | |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+ |
///   |                   RTP extension (OPTIONAL)                    | |
/// +>+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+ |
/// | |                          payload  ...                         | |
/// | |                               +-------------------------------+ |
/// | |                               | RTP padding   | RTP pad count | |
/// +>+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+<+
/// | ~                     SRTP MKI (OPTIONAL)                       ~ |
/// | +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+ |
/// | :                 authentication tag (RECOMMENDED)              : |
/// | +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+ |
/// |                                                                   |
/// +- Encrypted Portion*                      Authenticated Portion ---+
/// ```
#[derive(Debug, Clone)]
pub struct Srtp {
    
}