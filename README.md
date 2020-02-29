# QSV转FLV (Rust移植版)



学习Rust后的第一个项目，感谢原C#项目作者提供的针对QSV的解决方案

本程序仅支持QSV v2.0（可能无法处理某些较早的视频，大概16年之前）

用爱奇艺看电视时，突然想要收藏一些影视资源，网上去找相关的工具，但它们要么各种骗钱，要么不好用卡顿错帧，后面发现了一些不错的能够转换QSV的代码，暂时也能用。但总觉得他们的实现过于仓促，希望用Rust实现一个较好的实现

Rust确实不负我的希望，程序设计的体验要比C++好太多，同时又可以较大程度保证性能。



## 支持特性

* 仅学习用途（切勿将该项目用于非法盈利）
* 提供了简单的命令行前端

* 转换速度快
  * 在SSD下：大约是C#版本的2倍，与另一个由C++复刻的实现接近
  * 在HDD下：有时能快于C++复刻实现的2倍
  * 该结果是在Windows平台（后端是msvc）下试验得到的，也许LLVM后端可能会有更好的性能表现
* 无需生成临时文件
  * 可以减少不必要的写寿命损耗
  * 在读写相对较慢的HDD上，快于C++复刻的版本，样例情况中甚至接近快2倍
  * 本项目的性能瓶颈主要是相对更频繁的seek操作



执行顺序：

```rust
// 将QSV转换为FLV
fn convert_qsv_to_flv(qsv: &mut File, flv: &mut File) -> Result<()>
	/* [步骤] 检查QSV文件是否正确 */
	// 验证QSV格式
	fn validate_qsv_format(qsv: &mut File) -> Result<()>

	/* [步骤] 解析QSV文件中 */
	// 从QSV中解析得到每个TAG块的信息
	fn tag_blocks_from_qsv(qsv: &mut File) -> io::Result<FlvTagBlocks>
		// 将QSV文件seek至QSV的TAGS的起始处
		fn seek_qsv_to_start(qsv: &mut File) -> io::Result<()>
		// 使QSV文件的seek指针跳过元数据开头
		fn skip_qsv_metadata(qsv: &mut File) -> io::Result<()>
	// 从所有TAG块获取FLV的元数据
	fn meta_data_from_tag_blocks(qsv: &mut File, tags: &FlvTagBlocks) -> Result<MetaData>
		// 解析视频TAG块信息 (是否为关键帧, 视频编码ID)
		fn parse_video_tag(qsv: &mut File, tag: FlvTagBlock) -> Result<(bool, u8)>
		// 解析音频TAG块信息 (音频编码ID, 音频采样率, 音频采样大小, 音频是否为立体声)
		fn parse_audio_tag(qsv: &mut File, tag: FlvTagBlock) -> Result<(u8, u8, u8, bool)>
		// 从TAG块中读取时间戳
		fn get_time_stamp_from_tag(qsv: &mut File, tag: &FlvTagBlock) -> Result<i32>

	/* [步骤] 正在写入到FLV文件中 */
	// 根据已提取出的TAG块信息和FLV元数据，将QSV转换为FLV
	fn write_from_qsv_to_flv(qsv: &mut File, tags: FlvTagBlocks, meta: &MetaData, flv: &mut File)
		-> Result<()>
```



错误类型：

```rust
pub enum ErrorKind {
    Io(std::io::Error),      // 文件系统IO错误
    IncorrectQsvVersion,     // 错误的QSV版本：本程序无法处理
    IncorrectQsvFormat,      // 错误的QSV格式：不符合预期格式/该文件不是QSV
    QsvTagsIsEmpty,          // 该QSV文件TAG块数量小于1个
    MediaDurationIsTooShort, // 媒体时长过短
}
```



待办事项：

> * 性能探查：valgrind, qcachegrind
>
> * 
>
>   ```rust
>   trait FilePlus {
>       fn read_byte(&mut self) -> std::io::Result<Option<u8>>;
>       fn write_byte(&mut self, u8) -> io::Result<bool>;
>       fn tell(&mut self) -> std::io::Result<u64>;
>   }
>   impl FilePlus for File {
>       //...
>   }
>   ```
>
> * 性能优化：unsafe优化
>
> * 详细进度显示
>
> * 确保宿主机跨平台可移植性：跨系统、大小端、32位/64位
>
> * 完善注释、文档



## 计划支持

> * 管道输出/重定向输出
> * 改进FLV生成，提供更多可设定的FLV元数据参数（建议引入：flavors && nom）



测试样例：

>| 文件名称                     | 文件大小 | 存储介质         | 本项目[Rust]用时 | 其他版[C++]用时 |
>| ---------------------------- | -------- | ---------------- | ---------------- | --------------- |
>| 爱情公寓5第2集-蓝光1080P.qsv | 842MB    | 2.5英寸希捷机械  | 34.095s          | 65.914s         |
>| 爱情公寓5第1集-蓝光1080P.qsv | 895MB    | 2.5英寸Intel固态 | 14.048s          | 13.537s         |
>| Smooth Criminal.qsv          | 18MB     | 2.5英寸Intel固态 | 0.442s           | 0.143s          |
>
>* 

