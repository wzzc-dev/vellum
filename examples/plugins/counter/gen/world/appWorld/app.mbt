pub fn init_(ctx : @types.AppContext) -> Result[@types.ViewTree, @types.AppError] {
  @plugin_impl.app_init(ctx)
}

pub fn update(event : @types.AppEvent) -> Result[@types.ViewTree, @types.AppError] {
  @plugin_impl.app_update(event)
}

pub fn shutdown() -> Result[Unit, @types.AppError] {
  @plugin_impl.app_shutdown()
}
