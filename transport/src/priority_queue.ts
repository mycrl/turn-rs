// 推出事件处理函数
type Handle<T> = (values: Array<T>) => void

// 队列配置
// @param {loop} 超时检查运行间隔
export interface QueueOption {
    loop?: number
    ttl?: number
}

// 默认队列配置
const DefaultQueueOption: QueueOption = {
    loop: 5000,
    ttl: 6000
}

// 优先队列
// @class
export class PriorityQueue<T> {
    private option: QueueOption
    private queue: Array<number>
    private stack: Map<number, T>
    private handle?: Handle<T>
    
    // @constructor
    // @param {option?} 队列配置
    constructor (option?: QueueOption) {
        this.option = option || DefaultQueueOption
        this.stack = new Map()
        this.queue = []
        this.initialize()
    }
    
    // 推送新数据到队列
    // @param {index} 索引位置
    // @param {value} 数据
    public push (index: number, value: T) {
        let size = this.queue.length
        let max = null
        
        /**
         * 为了实现有序插入，每次写入的时候检查
         * 尾部最后的一 个值是否大于目前插入的值，
         * 如果大于尾部值则写入之后需要重新排序.
         */
        if (size > 0) {
            max = this.queue[size - 1]
        }
        
        /**
         * 将索引推送到队列.
         * 将数据放置到堆栈. 
         */
        this.queue.push(index)
        this.stack.set(index, value)
        
        /**
         * 尾部值大于先有值,
         * 重新排序.
         */
        if (max && max > index) {
            this.queue.sort((x, y) => {
                return x - y
            })
        }
        
        /**
         * 处理堆栈.
         * 只有在队列存在多条数据时才处理.
         */
        if (size > 0) {
            this.forwrad()   
        }
    }
    
    // 绑定推出事件
    // @param {handle} 推出事件处理函数
    public launch (handle: Handle<T>) {
        this.handle = handle
    }
    
    // 初始化
    private initialize () {
        
        /**
         * 在有生命周期要求的
         * 时候运行超时检查.
         */
        this.option.ttl && setInterval(() => {
            
        }, this.option.loop!)
    }
    
    // 推进堆栈处理
    private forwrad () {
        
    }
}
