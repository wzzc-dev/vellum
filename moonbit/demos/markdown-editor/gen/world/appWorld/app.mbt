pub fn init_(ctx : @types.AppContext) -> Result[@types.ViewTree, @types.AppError] {
  @demo.app_init(ctx)
}

pub fn update(event : @types.AppEvent) -> Result[@types.ViewTree, @types.AppError] {
  @demo.app_update(event)
}

pub fn shutdown() -> Result[Unit, @types.AppError] {
  @demo.app_shutdown()
}
