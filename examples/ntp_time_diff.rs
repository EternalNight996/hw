use e_utils::chrono::{parse_datetime_offset, DateTime, Duration};
use hw::os_more::net_manage::get_current_timezone;

#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  let target = "192.168.127.7:123";
  let res = ntp_client::Client::new().target(target)?.format(Some("%Y-%m-%d %H:%M:%S")).request()?;
  println!("时区：{}", get_current_timezone());
  let server_time = parse_datetime_offset(DateTime::from_timestamp(res.timestamp, 0).ok_or("?")?.naive_utc(), get_current_timezone()).ok_or("?")?;
  ntp_client::sync_systemtime(server_time)?;
  // 本地时间
  let local_time = ntp_client::Client::now_zh().ok_or("获取本地时间失败")?;
  // 计算时间差并验证
  let time_diff = server_time.signed_duration_since(local_time);
  let is_valid = time_diff.abs() <= Duration::minutes(10);

  println!("服务器时间: {}", server_time.format("%Y-%m-%d %H:%M:%S"));
  println!("本地时间: {}", local_time.format("%Y-%m-%d %H:%M:%S"));
  println!(
    "时间同步状态: {}",
    if is_valid {
      "✔️ 误差在10分钟内"
    } else {
      "❌ 误差超过10分钟"
    }
  );

  Ok(())
}
