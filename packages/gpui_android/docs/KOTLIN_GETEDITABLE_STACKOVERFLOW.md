# Kotlin 迁移 Bug：`getEditable()` 无限递归 → StackOverflowError

> 2026-08-07 实测（06_text_area 真机）。迁移 `GpuiActivity.java` → `GpuiActivity.kt` 时引入，
> 是**启动即闪退**类故障，任何一个使用 Kotlin 版 `GpuiActivity` 的 app 都会触发。

## 现象

- app 启动后立即崩溃，`logcat -b crash` 报告：

  ```
  E AndroidRuntime: FATAL EXCEPTION: main
  E AndroidRuntime: java.lang.StackOverflowError: stack size 8188KB
  E AndroidRuntime: 	at dev.gpui.mobile.GpuiActivity$GpuiInputView$onCreateInputConnection$1.getEditable(GpuiActivity.kt:194)
  E AndroidRuntime: 	at dev.gpui.mobile.GpuiActivity$GpuiInputView$onCreateInputConnection$1.getEditable(GpuiActivity.kt:194)
  E AndroidRuntime: 	...（同一帧无限重复）
  E AndroidRuntime: 	at android.view.inputmethod.BaseInputConnection.getTextBeforeCursor(...)
  E AndroidRuntime: 	at android.view.inputmethod.RemoteInputConnectionImpl...lambda$getTextBeforeCursor$7(...)
  ```

- 触发点：系统输入法一建立 `InputConnection` 就会调 `getTextBeforeCursor` →
  `getEditable()`，所以 **onCreate 后不久即崩**，表现为“启动即闪退”。

## 根因（Kotlin 合成属性遮蔽外层字段）

Kotlin 里匿名内部类（匿名 `BaseInputConnection` 子类）中的这段代码：

```kotlin
override fun getEditable(): Editable = editable
```

裸标识符 `editable` 被 Kotlin 解析成了 **`BaseInputConnection.getEditable()` 这个
JavaBean getter 对应的合成属性（synthetic property）**——即 `this.getEditable()`，
而不是外层 `GpuiInputView` 的同名字段。于是 `getEditable()` 调用自己 → 无限递归。

Java 版本没有这个歧义：Java 的词法查找优先取外层类的 `editable` 字段。

> 作用域要点：对 Java 基类 `getXxx()` 方法，Kotlin 会合成名为 `xxx` 的属性；
> 匿名类内裸写 `xxx` 优先命中合成属性，遮蔽外层成员字段。

## 修复

```kotlin
override fun getEditable(): Editable = this@GpuiInputView.editable
```

用 `this@GpuiInputView` label 显式限定到外层 inner class 的字段，绕开合成属性。

## 防护建议

- 匿名类里访问外层类字段时，若字段名与父类某 Java getter 同名，必须用
  `this@Outer.field` 限定，或直接改字段名，避免踩合成属性遮蔽。
- Kotlin 迁移时对“父类含 `get*` 方法”的类（如各种 InputConnection / Adapter）要
  格外检查裸属性访问。

## 验证记录

- 修复后 `assembleDebug` + `adb install -r` + `am start`：
  - `pidof gpui_learn.text_area_06` 返回存活 PID；
  - `logcat -b crash` 为空。
- 本 bug 与包名/配置无关，是所有 Kotlin `GpuiActivity` 应用的共同风险点。
