// Example application using Vellum GUI Framework with MoonBit

fn create_app() -> @gui.widget.Widget {
  // Create main layout
  let main_layout = @gui.Column::new([
    // Header
    @gui.Row::new([
      @gui.text("Vellum GUI Demo"),
    ]).padding(16.0).background(@gui.types.blue),

    // Content area
    @gui.Column::new([
      @gui.text("Welcome to Vellum GUI Framework!"),
      @gui.text("This demonstrates MoonBit + GPUI integration"),

      // Buttons section
      @gui.Column::new([
        @gui.text("Buttons:"),
        @gui.Row::new([
          @gui.button("Primary").variant(Primary),
          @gui.button("Secondary").variant(Secondary),
          @gui.button("Danger").variant(Danger),
        ]).gap(8.0),
      ]).padding(16.0),

      // Input section
      @gui.Column::new([
        @gui.text("Input:"),
        @gui.text_input("username", "Enter your name..."),
        @gui.checkbox("agree", "I agree to terms", false),
        @gui.toggle("enable", "Enable feature", false),
      ]).padding(16.0),

      // List section
      @gui.Column::new([
        @gui.text("List:"),
        @gui.list_view([
          @gui.text("Item 1"),
          @gui.text("Item 2"),
          @gui.text("Item 3"),
        ]),
      ]).padding(16.0),
    ]).gap(16.0).padding(24.0),

    // Footer
    @gui.Row::new([
      @gui.text("Powered by MoonBit + GPUI"),
    ]).padding(12.0).background(@gui.types.light_gray),
  ])

  main_layout
}

fn main() {
  let app = create_app()
  // In real usage, this would be passed to the GUI runtime
  // @gui.app.run(app)
}
