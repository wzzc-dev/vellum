// Form Demo - 表单输入与验证
// 展示表单状态管理、双向绑定、输入验证和提交处理

// 导入状态管理模块
use @gui.state.{State, Binding, autorun, computed, watch}
use @gui.types.{Color, color}
use @gui.container.{Column, Row, Form}
use @gui.text.{Text, Heading}
use @gui.button.{Button}
use @gui.input.{TextInput, Checkbox, Select, Slider}

// =============================================================================
// 表单数据模型
// =============================================================================

// 用户表单数据
struct UserForm {
  mut name: String
  mut email: String
  mut age: Int
  mut gender: Option[String]
  mut bio: String
  mut terms_accepted: Bool
  mut newsletter: Bool
}

impl UserForm {
  fn new() -> UserForm {
    {
      mut name: "",
      mut email: "",
      mut age: 18,
      mut gender: None,
      mut bio: "",
      mut terms_accepted: false,
      mut newsletter: false
    }
  }
  
  fn is_valid(self: UserForm) -> Bool {
    self.name.length() >= 2 &&
    self.is_email_valid() &&
    self.age >= 18 &&
    self.gender.is_some() &&
    self.terms_accepted
  }
  
  fn is_email_valid(self: UserForm) -> Bool {
    self.email.contains("@") && self.email.contains(".")
  }
  
  fn errors(self: UserForm) -> List[String] {
    let mut errors = []
    
    if self.name.length() < 2 {
      errors.push("Name must be at least 2 characters")
    }
    
    if !self.is_email_valid() {
      errors.push("Invalid email format")
    }
    
    if self.age < 18 {
      errors.push("Must be at least 18 years old")
    }
    
    if self.gender.is_none() {
      errors.push("Please select a gender")
    }
    
    if !self.terms_accepted {
      errors.push("Must accept terms and conditions")
    }
    
    errors
  }
  
  fn to_json(self: UserForm) -> String {
    "{
      \"name\": \"\{self.name}\",
      \"email\": \"\{self.email}\",
      \"age\": \{self.age},
      \"gender\": \"\{self.gender.unwrap_or(\"unknown\")}\",
      \"bio\": \"\{self.bio}\",
      \"newsletter\": \{self.newsletter}
    }"
  }
}

// =============================================================================
// 验证器
// =============================================================================

// 验证结果
struct ValidationResult {
  valid: Bool
  message: Option<String>
}

// 验证器函数类型
type Validator[T] = (T) -> ValidationResult

// 内置验证器
fn required(value: String) -> ValidationResult {
  if value.trim().length() > 0 {
    { valid: true, message: None }
  } else {
    { valid: false, message: Some("This field is required") }
  }
}

fn min_length(min: Int) -> Validator[String] {
  fn(value: String) -> ValidationResult {
    if value.length() >= min {
      { valid: true, message: None }
    } else {
      { valid: false, message: Some("Must be at least \{min} characters") }
    }
  }
}

fn max_length(max: Int) -> Validator[String] {
  fn(value: String) -> ValidationResult {
    if value.length() <= max {
      { valid: true, message: None }
    } else {
      { valid: false, message: Some("Must be at most \{max} characters") }
    }
  }
}

fn email_validator(value: String) -> ValidationResult {
  if value.contains("@") && value.contains(".") {
    { valid: true, message: None }
  } else {
    { valid: false, message: Some("Invalid email format") }
  }
}

fn range_validator(min: Int, max: Int) -> Validator[Int] {
  fn(value: Int) -> ValidationResult {
    if value >= min && value <= max {
      { valid: true, message: None }
    } else {
      { valid: false, message: Some("Must be between \{min} and \{max}") }
    }
  }
}

// 组合验证器
fn combine_validators(validators: List[Validator[String]]) -> Validator[String] {
  fn(value: String) -> ValidationResult {
    for validator in validators {
      let result = validator(value)
      if !result.valid {
        return result
      }
    }
    { valid: true, message: None }
  }
}

// =============================================================================
// 表单字段组件
// =============================================================================

// 输入字段（带标签和验证）
fn InputField(
  label: String,
  binding: Binding[String],
  placeholder: String,
  validator: Option[Validator[String]>
) -> Widget {
  let value = binding.get()
  
  let validation_result = match validator {
    Some(v) => v(value),
    None => { valid: true, message: None }
  }
  
  Column::new([
    // 标签
    Text::new(label)
      .font_size(14.0)
      .font_weight(Medium),
    
    // 输入框
    TextInput::new(label, placeholder)
      .value(value)
      .on_change(fn(text) {
        binding.set(text)
      })
      .border_color(
        if !validation_result.valid {
          color(0.8, 0.0, 0.0, 1.0)
        } else {
          color(0.8, 0.8, 0.8, 1.0)
        }
      )
      .padding(12.0),
    
    // 错误消息
    match validation_result.message {
      Some(msg) => Text::new(msg)
        .font_size(12.0)
        .color(color(0.8, 0.0, 0.0, 1.0)),
      None => Text::new("")
    },
  ])
  .gap(4.0)
}

// 选择字段
fn SelectField(
  label: String,
  binding: Binding[Option[String]],
  options: List[String],
  placeholder: String
) -> Widget {
  Column::new([
    Text::new(label)
      .font_size(14.0)
      .font_weight(Medium),
    
    Select::new(label, options, placeholder)
      .value(binding.get())
      .on_change(fn(value) {
        binding.set(Some(value))
      }),
  ])
  .gap(4.0)
}

// 滑块字段
fn SliderField(
  label: String,
  binding: Binding[Int],
  min: Int,
  max: Int
) -> Widget {
  Column::new([
    Row::new([
      Text::new(label)
        .font_size(14.0)
        .font_weight(Medium),
      
      Text::new("\{binding.get()}")
        .font_size(14.0)
        .color(color(0.3, 0.3, 0.8, 1.0)),
    ])
    .justify_content(SpaceBetween),
    
    Slider::new(label, binding.get().to_float(), min.to_float(), max.to_float())
      .on_change(fn(value) {
        binding.set(value.to_int())
      }),
  ])
  .gap(8.0)
}

// 复选框字段
fn CheckboxField(
  label: String,
  binding: Binding[Bool],
  description: Option<String>
) -> Widget {
  Row::new([
    Checkbox::new(label, "", binding.get())
      .on_change(fn(checked) {
        binding.set(checked)
      }),
    
    Column::new([
      Text::new(label)
        .font_size(14.0),
      
      match description {
        Some(desc) => Text::new(desc)
          .font_size(12.0)
          .color(color(0.5, 0.5, 0.5, 1.0)),
        None => Text::new("")
      },
    ])
    .gap(2.0),
  ])
  .gap(12.0)
  .align_items(Start)
}

// =============================================================================
// 主表单视图
// =============================================================================

fn UserFormView(form: UserForm) -> Widget {
  // 创建响应式状态
  let name_state = State::new(form.name)
  let email_state = State::new(form.email)
  let age_state = State::new(form.age)
  let gender_state = State::new(form.gender)
  let bio_state = State::new(form.bio)
  let terms_state = State::new(form.terms_accepted)
  let newsletter_state = State::new(form.newsletter)
  
  // 计算表单是否有效
  let is_valid = computed(fn() {
    name_state.get().length() >= 2 &&
    email_state.get().contains("@") &&
    age_state.get() >= 18 &&
    gender_state.get().is_some() &&
    terms_state.get()
  })
  
  // 计算错误列表
  let errors = computed(fn() {
    let mut error_list = []
    
    if name_state.get().length() < 2 {
      error_list.push("Name must be at least 2 characters")
    }
    
    if !email_state.get().contains("@") {
      error_list.push("Invalid email format")
    }
    
    if age_state.get() < 18 {
      error_list.push("Must be at least 18 years old")
    }
    
    if gender_state.get().is_none() {
      error_list.push("Please select a gender")
    }
    
    if !terms_state.get() {
      error_list.push("Must accept terms and conditions")
    }
    
    error_list
  })
  
  // 表单提交
  fn submit_form() -> Unit {
    if is_valid.get() {
      let form_data = {
        mut name: name_state.get(),
        mut email: email_state.get(),
        mut age: age_state.get(),
        mut gender: gender_state.get(),
        mut bio: bio_state.get(),
        mut terms_accepted: terms_state.get(),
        mut newsletter: newsletter_state.get()
      }
      
      @debug.log("Form submitted: \{form_data.to_json()}")
      @gui.dialog.alert("Success", "Form submitted successfully!")
    }
  }
  
  // 主视图
  Column::new([
    // 标题
    Heading::new("User Registration", 1)
      .text_align(Center)
      .padding(24.0),
    
    // 表单字段
    Form::new([
      // 姓名
      InputField(
        "Name",
        Binding::from_state(name_state),
        "Enter your name",
        Some(combine_validators([required, min_length(2)]))
      ),
      
      // 邮箱
      InputField(
        "Email",
        Binding::from_state(email_state),
        "Enter your email",
        Some(email_validator)
      ),
      
      // 年龄
      SliderField(
        "Age",
        Binding::from_state(age_state),
        1,
        100
      ),
      
      // 性别
      SelectField(
        "Gender",
        Binding::from_state(gender_state),
        ["Male", "Female", "Other", "Prefer not to say"],
        "Select gender"
      ),
      
      // 简介
      Column::new([
        Text::new("Bio")
          .font_size(14.0)
          .font_weight(Medium),
        
        TextInput::new("bio", "Tell us about yourself...")
          .value(bio_state.get())
          .on_change(fn(text) {
            bio_state.set(text)
          })
          .multiline(true)
          .max_length(500)
          .padding(12.0),
        
        Text::new("\{bio_state.get().length()}/500")
          .font_size(12.0)
          .color(color(0.5, 0.5, 0.5, 1.0)),
      ])
      .gap(4.0),
      
      // 条款和条件
      CheckboxField(
        "I accept the terms and conditions",
        Binding::from_state(terms_state),
        Some("You must accept the terms to continue")
      ),
      
      // 新闻订阅
      CheckboxField(
        "Subscribe to newsletter",
        Binding::from_state(newsletter_state),
        Some("Receive updates about new features")
      ),
    ])
    .gap(20.0)
    .padding(24.0),
    
    // 错误消息
    if errors.get().length() > 0 {
      Column::new(
        errors.get().map(fn(error) {
          Text::new("• \{error}")
            .font_size(12.0)
            .color(color(0.8, 0.0, 0.0, 1.0))
        })
      )
      .gap(4.0)
      .padding(12.0)
      .background(color(1.0, 0.95, 0.95, 1.0))
      .border_radius(8.0)
    },
    
    // 提交按钮
    Button::new("Submit")
      .on_click(submit_form)
      .variant(Primary)
      .enabled(is_valid.get())
      .padding(16.0, 48.0),
    
    // 表单预览
    Column::new([
      Text::new("Form Preview")
        .font_size(14.0)
        .font_weight(Medium),
      
      Text::new("Name: \{name_state.get()}")
        .font_size(12.0),
      Text::new("Email: \{email_state.get()}")
        .font_size(12.0),
      Text::new("Age: \{age_state.get()}")
        .font_size(12.0),
      Text::new("Gender: \{gender_state.get().unwrap_or(\"Not selected\")}")
        .font_size(12.0),
    ])
    .gap(4.0)
    .padding(16.0)
    .background(color(0.95, 0.95, 0.95, 1.0))
    .border_radius(8.0),
  ])
  .gap(24.0)
  .padding(32.0)
  .max_width(500.0)
}

// =============================================================================
// 主函数
// =============================================================================

pub fn main() -> Unit {
  let form = UserForm::new()
  let view = UserFormView(form)
  @gui.app.run(view)
}

// =============================================================================
// 测试
// =============================================================================

test("UserForm::is_valid returns false for empty form") {
  let form = UserForm::new()
  assert_eq!(form.is_valid(), false)
}

test("UserForm::is_valid returns true for valid form") {
  let form = UserForm::new()
  form.name = "John Doe"
  form.email = "john@example.com"
  form.age = 25
  form.gender = Some("Male")
  form.terms_accepted = true
  
  assert!(form.is_valid())
}

test("UserForm::errors returns correct errors") {
  let form = UserForm::new()
  let errors = form.errors()
  
  assert!(errors.length() > 0)
  assert!(errors.contains("Name must be at least 2 characters"))
}

test("email_validator validates correctly") {
  let valid = email_validator("test@example.com")
  assert!(valid.valid)
  
  let invalid = email_validator("invalid-email")
  assert_eq!(invalid.valid, false)
}

test("min_length validator works correctly") {
  let validator = min_length(3)
  
  let valid = validator("hello")
  assert!(valid.valid)
  
  let invalid = validator("hi")
  assert_eq!(invalid.valid, false)
}

test("range_validator validates correctly") {
  let validator = range_validator(0, 100)
  
  assert!(validator(50).valid)
  assert!(validator(0).valid)
  assert!(validator(100).valid)
  assert_eq!(validator(101).valid, false)
  assert_eq!(validator(-1).valid, false)
}
