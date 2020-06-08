# QSV转FLV (Rust移植版)

![license](http://img.shields.io/badge/license-MPL%20v2-blue.svg)
[![release](https://github.com/zhangzhilinx/qsv2flv/workflows/release/badge.svg)](https://github.com/zhangzhilinx/qsv2flv/releases)
[![version](https://img.shields.io/crates/l/qsv2flv/0.1.2.svg)](https://crates.io/crates/qsv2flv)
[![open issues](https://img.shields.io/github/issues-raw/zhangzhilinx/qsv2flv.svg)](https://github.com/zhangzhilinx/qsv2flv/issues)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-orange.svg)](https://github.com/zhangzhilinx/qsv2flv/pulls)

<br>

## 简介

一个可以将爱奇艺qsv格式视频转换为flv格式视频的简单命令行工具

学习Rust后的第一个项目，感谢原C#项目作者提供的针对QSV的解决方案，也欢迎大家fork或者提交PR，帮助完善功能或者修复bug

本程序仅支持QSV v2.0（可能无法处理某些较早的视频，比如说部分16年前的视频）

前段时间用爱奇艺看电视时，想要收藏一些影视资源，于是去网上找相关的转码工具。遗憾的是，有些工具要么各种骗钱，要么不好用卡顿错帧。后面发现了一些不错的能够转换QSV的代码，但总觉得他们的实现有些仓促，于是用Rust重写，并做出了改进

Rust的开发的体验确实不错，解决了很多C++的痛点，同时又可以较大程度保证运行性能

<br>

## 使用方法

命令行运行 *（最近实现了交叉编译，提供了多种系统平台下已编译好的程序）*

```out
qsv2flv 0.1.1
ZhangZhilin <corex_public@outlook.com>
A tool for converting QSV to FLV

USAGE:
    qsv2flv [FLAGS] <INPUT> <OUTPUT>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Print test information verbosely

ARGS:
    <INPUT>     Sets the input file to use
    <OUTPUT>    Sets the output file to use
```

<br>

## 功能特性

* 仅供学习用途（切勿将该项目用于非法盈利）

* 提供了简单的命令行前端交互

* 转换速度快
  
  * 在SSD硬盘下：效率大约是C#版本的2倍，与另一个由C++复刻的实现接近
  * 在HDD硬盘下：有时能快于C++复刻实现的2倍
  * 该结果是在Windows平台（后端为msvc）下试验得到的，也许LLVM后端可能会有更好的性能表现

* 不会生成临时文件
  
  * 减少固态不必要的写寿命损耗
  * 在读写相对较慢的HDD上，快于C++复刻的版本，部分情况下转码速度可达2倍
  * 实现思路的性能瓶颈主要是相对更频繁的随机seek操作

<br>

## 设计细节

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
    fn meta_data_from_tag_blocks(qsv: &mut File, tags: &[FlvTagBlock]) -> Result<MetaData>
        // 解析视频TAG块信息 (是否为关键帧, 视频编码ID)
        fn parse_video_tag(qsv: &mut File, tag: FlvTagBlock) -> Result<(bool, u8)>
        // 解析音频TAG块信息 (音频编码ID, 音频采样率, 音频采样大小, 音频是否为立体声)
        fn parse_audio_tag(qsv: &mut File, tag: FlvTagBlock) -> Result<(u8, u8, u8, bool)>
        // 从TAG块中读取时间戳
        fn get_time_stamp_from_tag(qsv: &mut File, tag: &FlvTagBlock) -> Result<i32>

    /* [步骤] 正在写入到FLV文件中 */
    // 根据已提取出的TAG块信息和FLV元数据，将QSV转换为FLV
    fn write_from_qsv_to_flv(qsv: &mut File, tags: &[FlvTagBlock], flv: &mut File, meta: &MetaData)
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

<br>

## 计划事项

### 计划任务

> * 检查输出路径是否已存在文件，询问覆盖，防止因File::create(...)导致意外覆盖
> 
> * 性能探查：valgrind, qcachegrind
> 
> * ```rust
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

### 计划特性

> * 支持管道输出/重定向输出
> * 改进FLV生成，提供更多可设定的FLV元数据参数（建议引入：flavors && nom）
> * 支持批量文件转码

<br>

## 性能测试

简单测试样例：

> | 文件名称                       | 文件大小  | 存储介质             | 本项目[Rust]用时 | 其他版[C++]用时 |
> | -------------------------- | ----- | ---------------- | ----------- | ---------- |
> | iPartment 5第2集-蓝光1080P.qsv | 842MB | 2.5英寸希捷**机械**    | 34.095s     | 65.914s    |
> | iPartment 5第1集-蓝光1080P.qsv | 895MB | 2.5英寸Intel**固态** | 14.048s     | 13.537s    |
> | Smooth Criminal.qsv        | 18MB  | 2.5英寸Intel**固态** | 0.442s      | 0.143s     |

<br>

## 更新记录

* v0.1.0
  
  * 更新日志
    
    - 第一个可用版本
    - 避免了临时文件的生成
  
  * 备注
    
    - 这是命令行程序，没有提供图形界面，但是使用方法并不难

* v0.1.1
  
  * 更新日志
    
    - 增加了磁盘缓存同步步骤，防止在某些意外情况下可移动磁盘数据丢失
    - 改动了错误处理代码，因为相关代码被新Rust版本标记为deprecated

* v0.1.2
  
  * 更新日志
    
    * 重构了少量代码，改善了代码风格，也因此改动了部分函数签名
  
  * 备注
    
    - 该版本代码行为上与v0.1.1一致，主要集中在代码风格、文档的改进
    - 将宏代码分离到单独模块: `macros.rs`
    - 提供了多种平台的编译结果
