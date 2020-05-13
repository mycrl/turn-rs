//! UDP转运模块.
//! 
//! ### 数据包定义
//! 
//! |  name  |  flag  |  message id  |  len   |  package id  |  package is end  |  data  |
//! |--------|--------|--------------|--------|--------------|------------------|--------|
//! |  len   |  1byte |  4byte       |  4byte |  1byte       |  1byte           |  x     |
//! |  data  |  x     |  x           |  x     |  x           |  0 | 1           |  x     |
//! 
//! * `flag` 标志位，用户自行定义.
//! * `message id` 消息ID，当前消息的序号.
//! * `len` 包长度.
//! * `package id` 包ID，当前包的序号.
//! * `package is end` 当前包是否已结束，未结束0，已结束1.
//! 
//! TODO:
//! 消息序号最大为u32, 对端应该自动溢出归0;

use bytes::{Bytes, BytesMut, BufMut};

/// UDP转运模块.
/// 
/// 处理音视频数据，将音视频数据包装
/// 成Udp包，并自动处理最大传输单元限制.
pub struct Transport {
    max_index: u32,
    index: u32,
    mtu: u32
}

impl Transport {
    /// 创建转运实例.
    /// 
    /// 初始化实例时应该指定最大传输单元大小.
    /// 注意：最大传输单元大小并不代表数据包最终大小，模块会写入一些控制
    /// 信息和序号附加到数据包，所以这里注意留出12个byte左右的冗余.
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use transport::Transport;
    ///
    /// Transport::new(1000);
    /// ```
    pub fn new(mtu: u32) -> Self {
        Self {
            mtu,
            index: 0,
            max_index: u32::max_value(),
        }
    }

    /// 创建Udp数据包.
    /// 
    /// 根据MTU自动分成多个Udp数据包，
    /// 并标记包序号和控制信息（是否结束）
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use transport::Transport;
    ///
    /// let mut transport = Transport::new(1000);
    /// transport.packet(b"hello");
    /// ```
    pub fn packet (&mut self, chunk: Bytes, flgs: u8) -> Vec<Bytes> {

        // MTU大小
        // 数据包大小
        let size = self.mtu as usize;
        let sum_size = chunk.len();

        // Udp包列表
        // 数据写入的偏移
        // 包序号
        let mut packets = Vec::new();
        let mut offset: usize = 0;
        let mut index: u8 = 0;

        // 无限循环
        // 直到分配完成
        loop {
            let mut package = BytesMut::new();

            // 为了避免指针的溢出访问
            // 如果超出范围，则只指定最大范围
            let end = if offset + size > sum_size {
                sum_size
            } else {
                offset + size
            };

            // 写入标记位
            // 写入消息序号
            // 写入包序号
            package.put_u8(flgs);
            package.put_u32(self.index);
            package.put_u32((end - offset) as u32);
            package.put_u8(index);

            // 检查包是否结束
            // 如果结束，写入结束位
            if sum_size == end {
                package.put_u8(1u8);
            } else {
                package.put_u8(0u8);
            }

            // 写入数据包范围数据
            // 讲数据包添加到列表
            package.extend_from_slice(&chunk[offset..end]);
            packets.push(package.freeze());

            // 如果已经写入结束，则跳出循环
            // 如果没有写入完成，调整偏移和序号
            if sum_size == end {
                break;
            } else {
                index += 1;
                offset = end;
            }
        }

        // 检查是否超出U32最大值
        // 如果超出最大值，则归零
        if self.index + 1 > self.max_index {
            self.index = 0;
        } else {
            self.index += 1;
        }

        packets
    }
}
