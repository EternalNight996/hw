fn main() -> e_utils::AnyResult<()> {
    // hw::core_temp::CoreTemp::clean_log()?;
    let path = hw::core_temp::CoreTemp::read_log_path()?;
    let datas = hw::core_temp::CoreTemp::parse_log(&path)?;
    println!("{:#?}", datas);
    Ok(())
}