// see https://github.com/ValveSoftware/GameNetworkingSockets/blob/master/src/steamnetworkingsockets/clientlib/SNP_WIRE_FORMAT.md

// ack, stop and ack_bits

// ack, delay


// 1001wnnn latest_received_pkt_num latest_received_delay [N] [ack_block_0 ... ack_block_N]
//     w    size of latest_received_pkt_num
//     0    16-bit
//     1    32-bit
//      nnn  number of blocks in this frame.
//      000 -
//      ... |
//      110 - use this number (as n-1)
//      111 - number of blocks is >6, explicit count byte N is present.  (Max 255 blocks)
fn parse_ack(buf) {
    let wnnn = buf.read_u8()?;
    let w    = wnnn & 0b1000;
    let  nnn = wnnn & 0b0111;

    let latest_received_pkt_num = if w == 0 {
        buf.read_u16()? as u32
    } else {
        buf.read_u32()?
    };
    let latest_received_delay = buf.read_u16()?;
    let num_blocks = if nnn == 0b111 {
        buf.read_u8()
    } else {
        nnn
    };

    for i in 0..num_blocks {
        let p = buf.read_u8()?;
        let a = p >> 8;
        let n = p & 0b1111;

        let a = if a & 0b1000 != 0 {
        } else {
        }
    }
}


enum Frame<'a> {
    StopWaiting(u64),
    Unreliable {
        message_num: u64,
        offset: Option<std::num::NonZeroU64>,
        payload: &'a [u8],
    },
}

/*
fn decode_payload(buf: &[u8]) -> std::io::Result<Frame> {
    use byteorder::{LE, ReadBytesExt};

    use std::num::NonZeroU64;
    use std::io::{Error, ErrorKind::{
        InvalidData,
    }};


    let mut p = &buf[..];

    let prefix = p.read_u8()?;

    match prefix & 0b11000000 {
        0b00000000 => {
            let e = (prefix & 0b100000) != 0;
            let m = (prefix & 0b010000) != 0;
            let o = (prefix & 0b001000) != 0;
            let sss = (prefix & 0b111) as usize;

            let message_num = unimplemented!(); //buf.read_varint();

            let offset = if o {
                //Some(buf.read_varint())
                NonZeroU64::new(unimplemented!())
            } else {
                None
            };

            let size = match sss {
                0b111 => buf.len() - (buf.len() - p.len()),
                0b101 | 0b110 =>
                    return Err(Error::new(InvalidData, "reserved size")),
                _ => {
                    (p.read_u8()? as usize) | sss << 8
                }
            };

            let is_first = offset.is_none();
            let is_last = e && offset.is_none();

            // FIXME: may panic
            let payload = &buf[p.len()..size];

            Ok(Frame::Unreliable { message_num, offset, payload })
        }
        0b01000000 => {
            unimplemented!("reliable")
        }
        0b10000000 => {
            // control

            //if prefix & 0b1000_0000 {
            //}
            let size = match prefix & 0b11 {
                0b00 => 1,
                0b01 => 2,
                0b10 => 3,
                0b11 => 8,
                _ => unsafe { std::hint::unreachable_unchecked() },
            };
            Ok(Frame::StopWaiting(p.read_uint::<LE>(size)?))
        }
        _ => Err(Error::new(InvalidData, "reserved prefix")),
    }
}
*/

/*
    00emosss [message_num] [offset] [size] data

    e: 0: There's more data after ths in the unreliable message.
          (Will be sent in another packet.)
       1: This is the last segment in the unreliable message.
    m: encoded size of message_num
       First segment in packet: message_num is absolute.  Only bottom N bits are sent.
           0: 16-bits
           1: 32-bits
       Subsequent segments: message number field is relative to previous
           0: no message number field follows, assume 1 greater than previous segment
           1: Var-int encoded offset from previous follows
           (NOTE: while encoding/decoding a packet, any reliable segment frames sent after unreliable data
           will *also* increment the current message number, even though the message number is *not*
           guaranteed to match that reliable segment.  Since in practice the message number often will
           match, making this encode/decode rule affords a small optimization.)
    o:  offset of this segment within message
        If first segment in packet, or message number differs from previous segment in packet:
            0: Zero offset, segment is first in message.  No offset field follows.
            1: varint-encoded offset follows
    sss: Size of data
        000-100: Append upper three bits to lower 8 bits in explicit size field,
                 which follows  (Max value is 0x4ff = 1279, which is larger than our MTU)
        101,110: Reserved
        111: This is the last frame, so message data extends to the end of the packet.
*/

/*
fn decode(mut buf: &[u8]) {
    let kind = buf.read_u8()
    let let mut nCurMsgNum = 0i64;
    let mut nDecodeReliablePos = 0i64;

    loop {
        if kind & 0xC0 == 0x00 {
            // Unreliable segment

            // Decode message number
            if nCurMsgNum == 0 {
                // First unreliable frame.
                // Message number is absolute, but only bottom N bits are sent
                static const char szUnreliableMsgNumOffset[] = "unreliable msgnum";
                int64 nLowerBits, nMask;
                if nFrameType & 0x10 {
                    READ_32BITU( nLowerBits, szUnreliableMsgNumOffset );
                    nMask = 0xffffffff;
                    nCurMsgNum = NearestWithSameLowerBits( (int32)nLowerBits, m_receiverState.m_nHighestSeenMsgNum );
                } else {
                    READ_16BITU( nLowerBits, szUnreliableMsgNumOffset );
                    nMask = 0xffff;
                    nCurMsgNum = NearestWithSameLowerBits( (int16)nLowerBits, m_receiverState.m_nHighestSeenMsgNum );
                }
                assert!((nCurMsgNum & nMask) == nLowerBits);

                if nCurMsgNum <= 0 {
                    DECODE_ERROR("SNP decode unreliable msgnum underflow.  %llx mod %llx, highest seen %llx",
                        (unsigned long long)nLowerBits, (unsigned long long)( nMask+1 ), (unsigned long long)m_receiverState.m_nHighestSeenMsgNum );
                }
                if std::abs(nCurMsgNum - m_receiverState.m_nHighestSeenMsgNum) > (nMask>>2) {
                    // We really should never get close to this boundary.
                    SpewWarningRateLimited( usecNow, "Sender sent abs unreliable message number using %llx mod %llx, highest seen %llx\n",
                        (unsigned long long)nLowerBits, (unsigned long long)( nMask+1 ), (unsigned long long)m_receiverState.m_nHighestSeenMsgNum );
                }
            } else {
                if nFrameType & 0x10 {
                    uint64 nMsgNumOffset;
                    READ_VARINT( nMsgNumOffset, "unreliable msgnum offset" );
                    nCurMsgNum += nMsgNumOffset;
                } else {
                    nCurMsgNum += 2;
                    //++nCurMsgNum += 1;
                    // wtf?
                }
            }
            if nCurMsgNum > m_receiverState.m_nHighestSeenMsgNum {
                m_receiverState.m_nHighestSeenMsgNum = nCurMsgNum;
            }

            // Decode segment offset in message
            let nOffset = 0u32;
            if nFrameType & 0x08 {
                READ_VARINT(nOffset, "unreliable data offset");
            }

            // Decode size, locate segment data
            READ_SEGMENT_DATA_SIZE(unreliable)
            assert!( cbSegmentSize > 0 ); // !TEST! Bogus assert, zero byte messages are OK.  Remove after testing

            // Receive the segment
            bool bLastSegmentInMessage = ( nFrameType & 0x20 ) != 0;
            SNP_ReceiveUnreliableSegment( nCurMsgNum, nOffset, pSegmentData, cbSegmentSize, bLastSegmentInMessage, usecNow );
        } else if nFrameType & 0xE0 == 0x40 {
            // Reliable segment

            // First reliable segment?
            if nDecodeReliablePos == 0 {
                // Stream position is absolute.  How many bits?
                static const char szFirstReliableStreamPos[] = "first reliable streampos";
                int64 nOffset, nMask;
                switch nFrameType & (3<<3) {
                    case 0<<3: READ_24BITU( nOffset, szFirstReliableStreamPos ); nMask = (1ll<<24)-1; break;
                    case 1<<3: READ_32BITU( nOffset, szFirstReliableStreamPos ); nMask = (1ll<<32)-1; break;
                    case 2<<3: READ_48BITU( nOffset, szFirstReliableStreamPos ); nMask = (1ll<<48)-1; break;
                    default: DECODE_ERROR( "Reserved reliable stream pos size" );
                }

                // What do we expect to receive next?
                int64 nExpectNextStreamPos = m_receiverState.m_nReliableStreamPos + len( m_receiverState.m_bufReliableStream );

                // Find the stream offset closest to that
                nDecodeReliablePos = ( nExpectNextStreamPos & ~nMask ) + nOffset;
                if nDecodeReliablePos + (nMask>>1) < nExpectNextStreamPos {
                    nDecodeReliablePos += nMask+1;
                    assert!( ( nDecodeReliablePos & nMask ) == nOffset );
                    assert!( nExpectNextStreamPos < nDecodeReliablePos );
                    assert!( nExpectNextStreamPos + (nMask>>1) >= nDecodeReliablePos );
                }
                if nDecodeReliablePos <= 0 {
                    DECODE_ERROR( "SNP decode first reliable stream pos underflow.  %llx mod %llx, expected next %llx",
                        (unsigned long long)nOffset, (unsigned long long)( nMask+1 ), (unsigned long long)nExpectNextStreamPos );
                }
                if std::abs( nDecodeReliablePos - nExpectNextStreamPos ) > (nMask>>2) {
                    // We really should never get close to this boundary.
                    SpewWarningRateLimited( usecNow, "Sender sent reliable stream pos using %llx mod %llx, expected next %llx\n",
                        (unsigned long long)nOffset, (unsigned long long)( nMask+1 ), (unsigned long long)nExpectNextStreamPos );
                }
            } else {
                // Subsequent reliable message encode the position as an offset from previous.
                static const char szOtherReliableStreamPos[] = "reliable streampos offset";
                int64 nOffset;
                switch ( nFrameType & (3<<3) )
                {
                    case 0<<3: nOffset = 0; break;
                    case 1<<3: READ_8BITU( nOffset, szOtherReliableStreamPos ); break;
                    case 2<<3: READ_16BITU( nOffset, szOtherReliableStreamPos ); break;
                    default: READ_32BITU( nOffset, szOtherReliableStreamPos ); break;
                }
                nDecodeReliablePos += nOffset;
            }

            // Decode size, locate segment data
            READ_SEGMENT_DATA_SIZE( reliable )

            // Ingest the segment.  If it seems fishy, abort processing of this packet
            // and do not acknowledge to the sender.
            if !SNP_ReceiveReliableSegment( nPktNum, nDecodeReliablePos, pSegmentData, cbSegmentSize, usecNow ) {
                return false;
            }

            // Advance pointer for the next reliable segment, if any.
            nDecodeReliablePos += cbSegmentSize;

            // Decoding rules state that if we have established a message number,
            // (from an earlier unreliable message), then we advance it.
            if nCurMsgNum > 0 {
                ++nCurMsgNum;
            }
        } else if kind & 0xFC == 0x80 {
            // Stop waiting

            int64 nOffset = 0;
            static const char szStopWaitingOffset[] = "stop_waiting offset";
            switch ( nFrameType & 3 )
            {
                case 0: READ_8BITU( nOffset, szStopWaitingOffset ); break;
                case 1: READ_16BITU( nOffset, szStopWaitingOffset ); break;
                case 2: READ_24BITU( nOffset, szStopWaitingOffset ); break;
                case 3: READ_64BITU( nOffset, szStopWaitingOffset ); break;
            }
            if ( nOffset >= nPktNum )
            {
                DECODE_ERROR( "stop_waiting pktNum %llu offset %llu", nPktNum, nOffset );
            }
            ++nOffset;
            int64 nMinPktNumToSendAcks = nPktNum-nOffset;
            if ( nMinPktNumToSendAcks == m_receiverState.m_nMinPktNumToSendAcks )
                continue;
            if ( nMinPktNumToSendAcks < m_receiverState.m_nMinPktNumToSendAcks )
            {
                // Sender must never reduce this number!  Check for bugs or bogus sender
                if ( nPktNum >= m_receiverState.m_nPktNumUpdatedMinPktNumToSendAcks )
                {
                    DECODE_ERROR( "SNP stop waiting reduced %lld (pkt %lld) -> %lld (pkt %lld)",
                        (long long)m_receiverState.m_nMinPktNumToSendAcks,
                        (long long)m_receiverState.m_nPktNumUpdatedMinPktNumToSendAcks,
                        (long long)nMinPktNumToSendAcks,
                        (long long)nPktNum
                        );
                }
                continue;
            }
            SpewType( steamdatagram_snp_log_packet+1, "  %s decode pkt %lld stop waiting: %lld (was %lld)",
                m_sName.c_str(),
                (long long)nPktNum,
                (long long)nMinPktNumToSendAcks, (long long)m_receiverState.m_nMinPktNumToSendAcks );
            m_receiverState.m_nMinPktNumToSendAcks = nMinPktNumToSendAcks;
            m_receiverState.m_nPktNumUpdatedMinPktNumToSendAcks = nPktNum;

            // Trim from the front of the packet gap list,
            // we can stop reporting these losses to the sender
            while ( !m_receiverState.m_mapPacketGaps.empty() )
            {
                auto h = m_receiverState.m_mapPacketGaps.begin();
                if ( h->first > m_receiverState.m_nMinPktNumToSendAcks )
                    break;
                if ( h->second.m_nEnd > m_receiverState.m_nMinPktNumToSendAcks )
                {
                    // Ug.  You're not supposed to modify the key in a map.
                    // I suppose that's legit, since you could violate the ordering.
                    // but in this case I know that this change is OK.
                    const_cast<int64 &>( h->first ) = m_receiverState.m_nMinPktNumToSendAcks;
                    break;
                }
                m_receiverState.m_mapPacketGaps.erase(h);
            }
        } else if kind & 0xF0 == 0x90 {
            // Ack

            // Parse latest received sequence number
            int64 nLatestRecvSeqNum;
            {
                static const char szAckLatestPktNum[] = "ack latest pktnum";
                int64 nLowerBits, nMask;
                if nFrameType & 0x40 {
                    READ_32BITU( nLowerBits, szAckLatestPktNum );
                    nMask = 0xffffffff;
                    nLatestRecvSeqNum = NearestWithSameLowerBits( (int32)nLowerBits, m_statsEndToEnd.m_nNextSendSequenceNumber );
                } else {
                    READ_16BITU( nLowerBits, szAckLatestPktNum );
                    nMask = 0xffff;
                    nLatestRecvSeqNum = NearestWithSameLowerBits( (int16)nLowerBits, m_statsEndToEnd.m_nNextSendSequenceNumber );
                }
                assert!( ( nLatestRecvSeqNum & nMask ) == nLowerBits );

                // Find the message number that is closes to
                if nLatestRecvSeqNum < 0 {
                    DECODE_ERROR( "SNP decode ack latest pktnum underflow.  %llx mod %llx, next send %llx",
                        (unsigned long long)nLowerBits, (unsigned long long)( nMask+1 ), (unsigned long long)m_statsEndToEnd.m_nNextSendSequenceNumber );
                }
                if std::abs( nLatestRecvSeqNum - m_statsEndToEnd.m_nNextSendSequenceNumber ) > (nMask>>2) {
                    // We really should never get close to this boundary.
                    SpewWarningRateLimited( usecNow, "Sender sent abs latest recv pkt number using %llx mod %llx, next send %llx\n",
                        (unsigned long long)nLowerBits, (unsigned long long)( nMask+1 ), (unsigned long long)m_statsEndToEnd.m_nNextSendSequenceNumber );
                }
                if nLatestRecvSeqNum >= m_statsEndToEnd.m_nNextSendSequenceNumber {
                    DECODE_ERROR( "SNP decode ack latest pktnum %lld (%llx mod %llx), but next outoing packet is %lld (%llx).",
                        (long long)nLatestRecvSeqNum, (unsigned long long)nLowerBits, (unsigned long long)( nMask+1 ),
                        (long long)m_statsEndToEnd.m_nNextSendSequenceNumber, (unsigned long long)m_statsEndToEnd.m_nNextSendSequenceNumber
                    );
                }
            }

            SpewType( steamdatagram_snp_log_packet+1, "  %s decode pkt %lld latest recv %lld\n",
                m_sName.c_str(),
                (long long)nPktNum, (long long)nLatestRecvSeqNum
            );

            // Locate our bookkeeping for this packet, or the latest one before it
            // Remember, we have a sentinel with a low, invalid packet number
            assert!( !m_senderState.m_mapInFlightPacketsByPktNum.empty() );
            auto inFlightPkt = m_senderState.m_mapInFlightPacketsByPktNum.upper_bound( nLatestRecvSeqNum );
            --inFlightPkt;
            assert!( inFlightPkt->first <= nLatestRecvSeqNum );

            // Parse out delay, and process the ping
            {
                uint16 nPackedDelay;
                READ_16BITU( nPackedDelay, "ack delay" );
                if ( nPackedDelay != 0xffff && inFlightPkt->first == nLatestRecvSeqNum )
                {
                    SteamNetworkingMicroseconds usecDelay = SteamNetworkingMicroseconds( nPackedDelay ) << k_nAckDelayPrecisionShift;
                    SteamNetworkingMicroseconds usecElapsed = usecNow - inFlightPkt->second.m_usecWhenSent;
                    assert!( usecElapsed >= 0 );

                    // Account for their reported delay, and calculate ping, in MS
                    int msPing = (usecElapsed - usecDelay) / 1000;

                    // Does this seem bogus?  (We allow a small amount of slop.)
                    // NOTE: A malicious sender could lie about this delay, tricking us
                    // into thinking that the real network latency is low, they are just
                    // delaying their replies.  This actually matters, since the ping time
                    // is an input into the rate calculation.  So we might need to
                    // occasionally send pings that require an immediately reply, and
                    // if those ping times seem way out of whack with the ones where they are
                    // allowed to send a delay, take action against them.
                    if msPing < -1 {
                        // Either they are lying or some weird timer stuff is happening.
                        // Either way, discard it.

                        SpewType( steamdatagram_snp_log_ackrtt, "%s decode pkt %lld latest recv %lld delay %lluusec INVALID ping %lldusec\n",
                            m_sName.c_str(),
                            (long long)nPktNum, (long long)nLatestRecvSeqNum,
                            (unsigned long long)usecDelay,
                            (long long)usecElapsed
                        );
                    } else {
                        // Clamp, if we have slop
                        if msPing < 0 {
                            msPing = 0;
                        }
                        m_statsEndToEnd.m_ping.ReceivedPing( msPing, usecNow );

                        // Spew
                        SpewType( steamdatagram_snp_log_ackrtt, "%s decode pkt %lld latest recv %lld delay %.1fms ping %.1fms\n",
                            m_sName.c_str(),
                            (long long)nPktNum, (long long)nLatestRecvSeqNum,
                            (float)(usecDelay * 1e-3 ),
                            (float)(usecElapsed * 1e-3 )
                        );
                    }
                }
            }

            // Parse number of blocks
            int nBlocks = nFrameType&7;
            if nBlocks == 7 {
                READ_8BITU( nBlocks, "ack num blocks" );
            }

            // If they actually sent us any blocks, that means they are fragmented.
            // We should make sure and tell them to stop sending us these nacks
            // and move forward.  This could be more robust, if we remember when
            // the last stop_waiting value we sent was, and when we sent it.
            if nBlocks > 0 {
                // Decrease flush delay the more blocks they send us.
                SteamNetworkingMicroseconds usecDelay = 250*1000 / nBlocks;
                m_receiverState.m_usecWhenFlushAck = std::min( m_receiverState.m_usecWhenFlushAck, usecNow + usecDelay );
            }

            // Process ack blocks, working backwards from the latest received sequence number.
            // Note that we have to parse all this stuff out, even if it's old news (packets older
            // than the stop_aiting value we sent), because we need to do that to get to the rest
            // of the packet.
            bool bAckedReliableRange = false;
            int64 nPktNumAckEnd = nLatestRecvSeqNum+1;
            while ( nBlocks >= 0 )
            {

                // Parse out number of acks/nacks.
                // Have we parsed all the real blocks?
                int64 nPktNumAckBegin, nPktNumNackBegin;
                if ( nBlocks == 0 )
                {
                    // Implicit block.  Everything earlier between the last
                    // NACK and the stop_waiting value is implicitly acked!
                    if ( nPktNumAckEnd <= m_senderState.m_nMinPktWaitingOnAck )
                        break;

                    nPktNumAckBegin = m_senderState.m_nMinPktWaitingOnAck;
                    nPktNumNackBegin = nPktNumAckBegin;
                    SpewType( steamdatagram_snp_log_packet+1, "  %s decode pkt %lld ack last block ack begin %lld\n",
                        m_sName.c_str(),
                        (long long)nPktNum, (long long)nPktNumAckBegin );
                }
                else
                {
                    uint8 nBlockHeader;
                    READ_8BITU( nBlockHeader, "ack block header" );

                    // Ack count?
                    int64 numAcks = ( nBlockHeader>> 4 ) & 7;
                    if ( nBlockHeader & 0x80 )
                    {
                        uint64 nUpperBits;
                        READ_VARINT( nUpperBits, "ack count upper bits" );
                        if ( nUpperBits > 100000 )
                            DECODE_ERROR( "Ack count of %llu<<3 is crazy", (unsigned long long)nUpperBits );
                        numAcks |= nUpperBits<<3;
                    }
                    nPktNumAckBegin = nPktNumAckEnd - numAcks;
                    if ( nPktNumAckBegin < 0 )
                        DECODE_ERROR( "Ack range underflow, end=%lld, num=%lld", (long long)nPktNumAckEnd, (long long)numAcks );

                    // Extended nack count?
                    int64 numNacks = nBlockHeader & 7;
                    if ( nBlockHeader & 0x08)
                    {
                        uint64 nUpperBits;
                        READ_VARINT( nUpperBits, "nack count upper bits" );
                        if ( nUpperBits > 100000 )
                            DECODE_ERROR( "Nack count of %llu<<3 is crazy", nUpperBits );
                        numNacks |= nUpperBits<<3;
                    }
                    nPktNumNackBegin = nPktNumAckBegin - numNacks;
                    if ( nPktNumNackBegin < 0 )
                        DECODE_ERROR( "Nack range underflow, end=%lld, num=%lld", (long long)nPktNumAckBegin, (long long)numAcks );

                    SpewType( steamdatagram_snp_log_packet+1, "  %s decode pkt %lld nack [%lld,%lld) ack [%lld,%lld)\n",
                        m_sName.c_str(),
                        (long long)nPktNum,
                        (long long)nPktNumNackBegin, (long long)( nPktNumNackBegin + numNacks ),
                        (long long)nPktNumAckBegin, (long long)( nPktNumAckBegin + numAcks )
                    );
                }

                // Process acks first.
                assert!( nPktNumAckBegin >= 0 );
                while ( inFlightPkt->first >= nPktNumAckBegin )
                {
                    assert!( inFlightPkt->first < nPktNumAckEnd );

                    // Scan reliable segments, and see if any are marked for retry or are in flight
                    for ( const SNPRange_t &relRange: inFlightPkt->second.m_vecReliableSegments )
                    {

                        // If range is present, it should be in only one of these two tables.
                        if ( m_senderState.m_listInFlightReliableRange.erase( relRange ) == 0 )
                        {
                            if ( m_senderState.m_listReadyRetryReliableRange.erase( relRange ) > 0 )
                            {

                                // When we put stuff into the reliable retry list, we mark it as pending again.
                                // But now it's acked, so it's no longer pending, even though we didn't send it.
                                m_senderState.m_cbPendingReliable -= int( relRange.length() );
                                assert!( m_senderState.m_cbPendingReliable >= 0 );

                                bAckedReliableRange = true;
                            }
                        }
                        else
                        {
                            bAckedReliableRange = true;
                            assert!( m_senderState.m_listReadyRetryReliableRange.count( relRange ) == 0 );
                        }
                    }

                    // Check if this was the next packet we were going to timeout, then advance
                    // pointer.  This guy didn't timeout.
                    if ( inFlightPkt == m_senderState.m_itNextInFlightPacketToTimeout )
                        ++m_senderState.m_itNextInFlightPacketToTimeout;

                    // No need to track this anymore, remove from our table
                    inFlightPkt = m_senderState.m_mapInFlightPacketsByPktNum.erase( inFlightPkt );
                    --inFlightPkt;
                    assert!( !m_senderState.m_mapInFlightPacketsByPktNum.empty() );
                }

                // Ack of in-flight end-to-end stats?
                if ( nPktNumAckBegin <= m_statsEndToEnd.m_pktNumInFlight && m_statsEndToEnd.m_pktNumInFlight < nPktNumAckEnd )
                    m_statsEndToEnd.InFlightPktAck( usecNow );

                // Process nacks.
                assert!(nPktNumNackBegin >= 0);
                while inFlightPkt->first >= nPktNumNackBegin {
                    assert!( inFlightPkt->first < nPktNumAckEnd );
                    SNP_SenderProcessPacketNack( inFlightPkt->first, inFlightPkt->second, "NACK" );

                    // We'll keep the record on hand, though, in case an ACK comes in
                    --inFlightPkt;
                }

                // Continue on to the the next older block
                nPktNumAckEnd = nPktNumNackBegin;
                --nBlocks;
            }

            // Should we check for discarding reliable messages we are keeping around in case
            // of retransmission, since we know now that they were delivered?
            if bAckedReliableRange {
                m_senderState.RemoveAckedReliableMessageFromUnackedList();

                // Spew where we think the peer is decoding the reliable stream
                if g_eSteamDatagramDebugOutputDetailLevel <= k_ESteamNetworkingSocketsDebugOutputType_Debug {
                    int64 nPeerReliablePos = m_senderState.m_nReliableStreamPos;
                    if ( !m_senderState.m_listInFlightReliableRange.empty() )
                        nPeerReliablePos = std::min( nPeerReliablePos, m_senderState.m_listInFlightReliableRange.begin()->first.m_nBegin );
                    if ( !m_senderState.m_listReadyRetryReliableRange.empty() )
                        nPeerReliablePos = std::min( nPeerReliablePos, m_senderState.m_listReadyRetryReliableRange.begin()->first.m_nBegin );

                    SpewType( steamdatagram_snp_log_packet+1, "  %s decode pkt %lld peer reliable pos = %lld\n",
                        m_sName.c_str(),
                        (long long)nPktNum, (long long)nPeerReliablePos );
                }
            }

            // Check if any of this was new info, then advance our stop_waiting value.
            if nLatestRecvSeqNum > m_senderState.m_nMinPktWaitingOnAck {
                SpewType( steamdatagram_snp_log_packet, "  %s updating min_waiting_on_ack %lld -> %lld\n",
                    m_sName.c_str(),
                    (long long)m_senderState.m_nMinPktWaitingOnAck, (long long)nLatestRecvSeqNum );
                m_senderState.m_nMinPktWaitingOnAck = nLatestRecvSeqNum;
                //m_senderState.m_usecWhenAdvancedMinPktWaitingOnAck = usecNow;
            }
        }
        else
        {
            DECODE_ERROR( "Invalid SNP frame lead byte 0x%02x", nFrameType );
        }
    }
}
*/
