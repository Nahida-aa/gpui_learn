# Android IME 输入闪退排查记录

本例（`05_android_input`）在真机上跑 GPUI 文本输入时，曾出现**一输入普通字符就闪退**的问题。
本文记录现象、根因与修复，供后续维护参考。

## 现象

- 键盘能正常弹出（`windowSoftInputMode="stateVisible|adjustResize"` + `SHOW_FORCED` 已解决显示问题）。
- 输入**退格**（`KEYCODE_DEL`）和**换行**不崩溃。
- 输入**任意普通字符**（如 `a`、`hello`）立即闪退，logcat 报：

  ```
  java.lang.UnsatisfiedLinkError: No implementation found for void
  dev.gpui.mobile.GpuiActivity.nativeCommitText(java.lang.String)
  (tried Java_dev_gpui_mobile_GpuiActivity_nativeCommitText and
  Java_dev_gpui_mobile_GpuiActivity_nativeCommitText__Ljava_lang_String_2)
  ```

  调用栈：`GpuiActivity$GpuiInputView$1.commitText` → `nativeCommitText(Native Method)`。

## 根因

`GpuiActivity extends NativeActivity`。`NativeActivity` 框架在 `onCreate` 时把
`libinput_05.so`（`android.app.lib_name` 指定的库）加载进 **framework 类加载器** 的命名空间。

而 `nativeCommitText` / `nativeSetComposingText` / `nativeFinishComposingText` /
`nativeDeleteSurroundingText` 这些 native 方法**声明在应用类 `GpuiActivity` 上**，JNI 在解析时会按
**应用类加载器** 去查找已加载的库 —— 这个命名空间并没有看到 framework 那次加载。

所以：

- 符号其实**一直存在于 `.so` 中**（`llvm-nm -D libinput_05.so | grep nativeCommitText` 可验证）。
- 但按应用类加载器解析时找不到，于是第一次真正的 `commitText` 抛 `UnsatisfiedLinkError` 并强制崩溃。

### 为什么退格 / 换行不崩，字符输入崩

- 退格（`deleteSurroundingText`）和换行走的是 Android 框架**自身处理**的按键事件路径，
  不依赖应用类声明的 native 方法，所以那次 framework 加载够用，不崩。
- 普通字符走 `InputConnection.commitText → nativeCommitText`，必须解析应用类上声明的
  native 方法，于是崩。

## 修复

在 `GpuiActivity.onCreate` 里，用**应用类加载器**显式再 `System.loadLibrary` 一次，把库注册到正确的
命名空间：

```java
private static void ensureNativeLibLoaded(Context ctx) {
    // ... 读取 manifest 的 android.app.lib_name 元数据得到库名 ...
    System.loadLibrary(libName);
}
```

库名从 `AndroidManifest.xml` 的 `android.app.lib_name` 元数据读取（值为 `input_05`），
使这一份 vendored 的 `GpuiActivity.java` 对所有生成的应用通用，不写死库名。

该修复已提交（commit `acbc43f`），位于 `crates/gpui-android/android/src/main/java/dev/gpui/mobile/GpuiActivity.java`。
由于各生成项目（`gen/android/app/build.gradle.kts`）以源码目录方式直接引用该 Java 文件
（`java.srcDir("../../../../../crates/gpui-android/android/src/main/java")`），改一处即对所有 Android 例子生效。

## 验证

真机实测：输入 `hello123 world`、`abc` + 退格 + `xyz` 等，退格/换行/普通字符/组合输入均不再闪退，
进程持续存活，四个 IME native 方法均被正常调用。

## 后续排查提示

再遇到 `UnsatisfiedLinkError ... tried Java_...` 类崩溃时，先怀疑**双类加载器命名空间**问题，
而不是去查 Rust 侧符号是否导出。验证符号是否在 `.so`：

```bash
llvm-nm -D <apk 解包或 target 下的> libinput_05.so | grep Java_dev_gpui
```
