use sysinfo::{ProcessorExt, System, SystemExt};
use bytes::{BytesMut, BufMut};

/// 性能
/// 
/// 计算当前实例环境的性能情况.
#[derive(Clone, Copy, Debug)]
pub struct Performance {
    average: u8,
    memory: u8,
    cpu: u8,
}

impl Performance {
    /// 新建性能计算实例
    /// 
    /// # Examples
    ///
    /// ```no_run
    /// use performance::Performance;
    ///
    /// let fraction = Performance::new();
    /// ```
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            cpu: Self::process(&system),
            memory: Self::memory(&system),
            average: Self::load_average(&system),
        }
    }

    /// 将性能数据转换成字节组
    /// 此功能用于将数据在网络中传输.
    /// /// # Examples
    ///
    /// ```no_run
    /// use performance::Performance;
    ///
    /// let fraction = Performance::new();
    /// fraction.as_bytes();
    /// ```
    pub fn as_bytes(self) -> BytesMut {
        let mut packet = BytesMut::new();
        packet.put_u8(self.cpu);
        packet.put_u8(self.memory);
        packet.put_u8(self.average);
        packet
    }

    /// 计算内存占用百分比
    /// 
    /// 只计算实际占用情况，
    /// 不计算swap的占用情况.
    #[allow(dead_code)]
    fn memory(system: &System) -> u8 {
        let totel = system.get_total_memory() as f32;
        let free = system.get_free_memory() as f32;
        let value = (free / totel) * 100f32;
        value as u8
    }

    /// 获取负载情况
    /// 
    /// 根据不同的权重，
    /// 计算出3个采样的平均值.
    #[allow(dead_code)]
    fn load_average(system: &System) -> u8 {
        let average = system.get_load_average();
        let mut count = 0f64;
        count += average.one * 100f64 * 0.1;
        count += average.five * 100f64 * 0.3;
        count += average.fifteen * 100f64 * 0.6;
        let value = (count / 300f64) as f32;
        value as u8
    }

    /// 计算处理器百分比
    /// 
    /// 获取所有核心的处理器百分比，
    /// (核心数 * 100) / (负载 * 核心数)
    #[allow(dead_code)]
    fn process(system: &System) -> u8 {
        let processors = system.get_processors();
        let count = (processors.len() * 100) as f32;
        let mut average = 0f32;
        processors
            .iter()
            .enumerate()
            .for_each(|(_, x)| {
                average += x.get_cpu_usage();
            });
        let value = (average / count) * 100f32;
        value as u8
    }
}
