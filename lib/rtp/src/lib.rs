//! ## RTP: A Transport Protocol for Real-Time Applications
//!
//! This project specifies the real-time transport protocol (RTP),
//! which provides end-to-end delivery services for data with real-time
//! characteristics, such as interactive audio and video.  Those services
//! include payload type identification, sequence numbering, timestamping
//! and delivery monitoring.  Applications typically run RTP on top of
//! UDP to make use of its multiplexing and checksum services; both
//! protocols contribute parts of the transport protocol functionality.
//! However, RTP may be used with other suitable underlying network or
//! transport protocols. RTP supports data transfer to multiple 
//! destinations using multicast distribution if provided by the
//! underlying network.
//! 
//! Note that RTP itself does not provide any mechanism to ensure timely
//! delivery or provide other quality-of-service guarantees, but relies
//! on lower-layer services to do so.  It does not guarantee delivery or
//! prevent out-of-order delivery, nor does it assume that the underlying
//! network is reliable and delivers packets in sequence.  The sequence
//! numbers included in RTP allow the receiver to reconstruct the
//! sender's packet sequence, but sequence numbers might also be used to
//! determine the proper location of a packet, for example in video
//! decoding, without necessarily decoding packets in sequence.
//! 
//! While RTP is primarily designed to satisfy the needs of multi-
//! participant multimedia conferences, it is not limited to that
//! particular application.  Storage of continuous data, interactive
//! distributed simulation, active badge, and control and measurement
//! applications may also find RTP applicable.
//!

pub mod header;
pub mod extension;

use header::Header;
use extension::Extension;

/// Secure RTP
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
pub struct SecureRtp {
    pub header: Header,
    pub extension: Option<Extension>,
}