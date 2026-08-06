package dev.gpui.mobile

import android.app.NativeActivity
import android.content.Context
import android.content.Intent
import android.graphics.Rect
import android.os.Bundle
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.text.Editable
import android.text.SpannableStringBuilder
import android.util.Base64
import android.util.Log
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityNodeProvider
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import org.json.JSONObject
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * NativeActivity host used by GPUI Android applications.
 *
 * The small editor view supplies the InputConnection that NativeActivity
 * itself lacks, enabling full commit/composition callbacks for software IMEs.
 * Credential helpers keep encrypted payloads in app-private preferences and
 * the encryption key in AndroidKeyStore.
 *
 * 本文件由 GpuiActivity.java 翻译为 Kotlin（见 git 历史可查 Java 版）。
 * 方法名 / 类名 / JNI 签名保持不变，Rust 侧调用面不受影响。
 */
class GpuiActivity : NativeActivity() {

    private lateinit var inputView: GpuiInputView

    override fun onCreate(state: Bundle?) {
        ensureNativeLibLoaded(this)
        super.onCreate(state)
        inputView = GpuiInputView(this)
        inputView.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_YES
        val params = FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        )
        params.gravity = Gravity.BOTTOM or Gravity.END
        addContentView(inputView, params)
        dispatchDeepLink(intent)
    }

    override fun onNewIntent(injectedInt: Intent) {
        super.onNewIntent(injectedInt)
        intent = injectedInt
        dispatchDeepLink(injectedInt)
    }

    private fun dispatchDeepLink(intent: Intent?) {
        intent?.dataString?.let { nativeOnDeepLink(it) }
    }

    /** Called by Rust; safely moves focus and IME operations to the UI thread. */
    fun gpuiShowKeyboard(inputType: Int) {
        runOnUiThread {
            inputView.inputType = inputType
            // post 到 view 的消息队列：确保 showSoftInput 在 inputView 已 attach 到
            // window、完成布局之后执行。过早调用会因为没有有效 window token 被系统丢弃。
            inputView.post {
                inputView.requestFocus()
                val manager = getSystemService(InputMethodManager::class.java)
                if (manager != null) {
                    // SHOW_FORCED 而非 SHOW_IMPLICIT：后者在窗口尚未完全布局或紧跟
                    // restartInput 调用时经常被系统静默丢弃，导致软键盘不弹出。
                    manager.showSoftInput(inputView, InputMethodManager.SHOW_FORCED)
                }
            }
        }
    }

    /** Called by Rust; safely hides the software keyboard on the UI thread. */
    fun gpuiHideKeyboard() {
        runOnUiThread {
            val manager = getSystemService(InputMethodManager::class.java)
            if (manager != null) {
                manager.hideSoftInputFromWindow(inputView.windowToken, 0)
            }
            inputView.clearFocus()
        }
    }

    /** Called by Rust after an AccessKit tree update. */
    fun gpuiAccessibilityChanged() {
        runOnUiThread {
            inputView.sendAccessibilityEvent(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED)
        }
    }

    /** Encrypt and persist a credential using a non-exportable AES-GCM key. */
    fun gpuiWriteCredential(alias: String, secret: ByteArray) {
        val key = getOrCreateKey(alias)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val encoded = Base64.encodeToString(cipher.iv, Base64.NO_WRAP) + ":" +
            Base64.encodeToString(cipher.doFinal(secret), Base64.NO_WRAP)
        getSharedPreferences(CREDENTIAL_PREFS, MODE_PRIVATE)
            .edit()
            .putString(alias, encoded)
            .apply()
    }

    /** Decrypt a credential, or return null when no value exists. */
    fun gpuiReadCredential(alias: String): ByteArray? {
        val encoded = getSharedPreferences(CREDENTIAL_PREFS, MODE_PRIVATE)
            .getString(alias, null) ?: return null
        val parts = encoded.split(":", limit = 2)
        if (parts.size != 2) {
            throw IllegalStateException("Malformed GPUI credential")
        }
        val store = KeyStore.getInstance(KEYSTORE)
        store.load(null)
        val key = store.getKey(alias, null) as? SecretKey ?: return null
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            key,
            GCMParameterSpec(128, Base64.decode(parts[0], Base64.NO_WRAP))
        )
        return cipher.doFinal(Base64.decode(parts[1], Base64.NO_WRAP))
    }

    /** Delete both the encrypted payload and its AndroidKeyStore key. */
    fun gpuiDeleteCredential(alias: String) {
        getSharedPreferences(CREDENTIAL_PREFS, MODE_PRIVATE).edit().remove(alias).apply()
        val store = KeyStore.getInstance(KEYSTORE)
        store.load(null)
        if (store.containsAlias(alias)) {
            store.deleteEntry(alias)
        }
    }

    private fun getOrCreateKey(alias: String): SecretKey {
        val store = KeyStore.getInstance(KEYSTORE)
        store.load(null)
        store.getKey(alias, null)?.let { return it as SecretKey }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .build()
        )
        return generator.generateKey()
    }

    private inner class GpuiInputView(context: Context) : View(context) {
        private val editable: Editable = SpannableStringBuilder()
        private val accessibilityProvider = GpuiAccessibilityProvider(this)
        var inputType: Int = EditorInfo.TYPE_CLASS_TEXT

        init {
            isFocusable = true
            isFocusableInTouchMode = true
        }

        override fun onCheckIsTextEditor(): Boolean = true

        override fun getAccessibilityNodeProvider(): AccessibilityNodeProvider = accessibilityProvider

        override fun onCreateInputConnection(attributes: EditorInfo): InputConnection {
            attributes.inputType = inputType
            // 多行输入：inputType 带 TYPE_TEXT_FLAG_MULTI_LINE 时，多数输入法会向
            // commitText("\n") 提交回车；但部分系统输入法仍会把回车当成 IME action
            // 触发 performEditorAction 而不提交 \n。为兜底，下面重写了
            // performEditorAction，在回车时也 nativeCommitText("\n")，确保换行必发生。
            // 这里不声明 IME_FLAG_NO_ENTER_ACTION / IME_ACTION_NONE：前者反而可能让
            // 某些输入法把回车当“无动作”，后者曾导致回车不提交（见 05 的调试记录）。
            attributes.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI
            attributes.initialSelStart = editable.length
            attributes.initialSelEnd = editable.length
            return object : BaseInputConnection(this, true) {
                override fun getEditable(): Editable = this@GpuiInputView.editable

                override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
                    Log.i("text_area_07", "IME commitText: \"$text\"")
                    nativeCommitText(text?.toString() ?: "")
                    return super.commitText(text, newCursorPosition)
                }

                // 软键盘把回车当成“完成/发送/下一个”等 IME action 时走这里
                // （单行 inputType 下常见）。这里**不**直接提交文本：把回车作为一次
                // 完整的 keystroke（DOWN+UP）发给 Rust，经 nativeKeyEvent →
                // on_key_event → observe_keystrokes。
                // 是否换行由聚焦的 GPUI 组件决定（Editor 插入 \n；纯 IME 输入框可不处理）。
                override fun performEditorAction(editorAction: Int): Boolean {
                    Log.i("gpui", "IME performEditorAction: $editorAction")
                    nativeKeyEvent(KeyEvent.KEYCODE_ENTER, KeyEvent.ACTION_DOWN, 0)
                    nativeKeyEvent(KeyEvent.KEYCODE_ENTER, KeyEvent.ACTION_UP, 0)
                    return true
                }

                // 输入法把某些键当硬件键下发时走 sendKeyEvent（多行 inputType 下的回车、
                // 退格等）。同样**不**拦截成文本，而是转发成 keystroke 给 Rust：
                // 列表能看到 enter/backspace，且由 GPUI 组件自行决定作用。
                override fun sendKeyEvent(event: KeyEvent): Boolean {
                    Log.i(TAG, "IME sendKeyEvent: code=${event.keyCode} action=${event.action}")
                    nativeKeyEvent(event.keyCode, event.action, event.metaState)
                    return true
                }

                override fun setComposingText(text: CharSequence, newCursorPosition: Int): Boolean {
                    nativeSetComposingText(text?.toString() ?: "")
                    return super.setComposingText(text, newCursorPosition)
                }

                override fun finishComposingText(): Boolean {
                    nativeFinishComposingText()
                    return super.finishComposingText()
                }

                override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
                    nativeDeleteSurroundingText(beforeLength, afterLength)
                    return super.deleteSurroundingText(beforeLength, afterLength)
                }
            }
        }
    }

    private inner class GpuiAccessibilityProvider(private val host: View) : AccessibilityNodeProvider() {

        private val virtualIds = HashMap<Long, Int>()
        private val nodeIds = HashMap<Int, Long>()
        private var nextVirtualId = 1
        private var accessibilityFocusedId: Int = View.NO_ID

        private fun snapshot(): JSONObject {
            val json = nativeAccessibilitySnapshot()
            return JSONObject(if (json.isNullOrEmpty()) "{}" else json)
        }

        private fun virtualId(nodeId: Long): Int {
            virtualIds[nodeId]?.let { return it }
            val id = nextVirtualId++
            virtualIds[nodeId] = id
            nodeIds[id] = nodeId
            return id
        }

        private fun findNode(snapshot: JSONObject, nodeId: Long): org.json.JSONObject? {
            val nodes = snapshot.optJSONArray("nodes") ?: return null
            for (index in 0 until nodes.length()) {
                val node = nodes.getJSONObject(index)
                if (node.optLong("id", -1) == nodeId) return node
            }
            return null
        }

        private fun findParent(snapshot: JSONObject, nodeId: Long): Long? {
            val nodes = snapshot.optJSONArray("nodes") ?: return null
            for (index in 0 until nodes.length()) {
                val candidate = nodes.getJSONObject(index)
                val children = candidate.optJSONArray("children") ?: continue
                for (child in 0 until children.length()) {
                    if (children.getLong(child) == nodeId) return candidate.getLong("id")
                }
            }
            return null
        }

        override fun createAccessibilityNodeInfo(virtualViewId: Int): AccessibilityNodeInfo? {
            return try {
                val snapshot = snapshot()
                if (virtualViewId == HOST_ID) {
                    val info = AccessibilityNodeInfo.obtain(host)
                    info.packageName = host.context.packageName
                    info.className = GpuiActivity::class.java.name
                    info.setSource(host)
                    if (!snapshot.isNull("root")) {
                        info.addChild(host, virtualId(snapshot.getLong("root")))
                    }
                    return info
                }

                val nodeId = nodeIds[virtualViewId] ?: return null
                val node = findNode(snapshot, nodeId) ?: return null

                val info = AccessibilityNodeInfo.obtain()
                info.packageName = host.context.packageName
                info.setSource(host, virtualViewId)
                val parent = findParent(snapshot, nodeId)
                if (parent == null) {
                    info.setParent(host)
                } else {
                    info.setParent(host, virtualId(parent))
                }

                val role = node.optString("role", "Unknown")
                info.className = classNameForRole(role)
                val label = node.optString("label", "")
                val value = node.optString("value", "")
                val description = node.optString("description", "")
                info.contentDescription = if (label.isEmpty()) description else label
                if (value.isNotEmpty()) info.setText(value)
                info.isEnabled = !node.optBoolean("disabled", false)

                val bounds = node.optJSONArray("bounds")
                if (bounds != null && bounds.length() == 4) {
                    val rect = Rect(
                        Math.floor(bounds.getDouble(0)).toInt(),
                        Math.floor(bounds.getDouble(1)).toInt(),
                        Math.ceil(bounds.getDouble(2)).toInt(),
                        Math.ceil(bounds.getDouble(3)).toInt()
                    )
                    info.setBoundsInParent(rect)
                    val location = IntArray(2)
                    host.getLocationOnScreen(location)
                    rect.offset(location[0], location[1])
                    info.setBoundsInScreen(rect)
                }

                val children = node.optJSONArray("children")
                if (children != null) {
                    for (index in 0 until children.length()) {
                        info.addChild(host, virtualId(children.getLong(index)))
                    }
                }

                val clickable = node.optBoolean("click", false)
                val focusable = node.optBoolean("focus", false) || clickable
                info.isClickable = clickable
                info.isFocusable = focusable
                if (clickable) info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_CLICK)
                if (focusable) info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_FOCUS)
                info.addAction(
                    if (virtualViewId == accessibilityFocusedId)
                        AccessibilityNodeInfo.AccessibilityAction.ACTION_CLEAR_ACCESSIBILITY_FOCUS
                    else
                        AccessibilityNodeInfo.AccessibilityAction.ACTION_ACCESSIBILITY_FOCUS
                )
                if (node.optBoolean("increment", false)) {
                    info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SCROLL_FORWARD)
                }
                if (node.optBoolean("decrement", false)) {
                    info.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SCROLL_BACKWARD)
                }
                info.isAccessibilityFocused = virtualViewId == accessibilityFocusedId
                info
            } catch (_: Exception) {
                null
            }
        }

        override fun performAction(virtualViewId: Int, action: Int, arguments: Bundle?): Boolean {
            val nodeId = nodeIds[virtualViewId] ?: return false
            return when (action) {
                AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS -> {
                    accessibilityFocusedId = virtualViewId
                    sendVirtualEvent(virtualViewId, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED)
                    nativeAccessibilityAction(nodeId, ACTION_FOCUS)
                }
                AccessibilityNodeInfo.ACTION_CLEAR_ACCESSIBILITY_FOCUS -> {
                    accessibilityFocusedId = HOST_ID
                    sendVirtualEvent(virtualViewId, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED)
                    true
                }
                AccessibilityNodeInfo.ACTION_CLICK -> nativeAccessibilityAction(nodeId, ACTION_CLICK)
                AccessibilityNodeInfo.ACTION_FOCUS -> nativeAccessibilityAction(nodeId, ACTION_FOCUS)
                AccessibilityNodeInfo.ACTION_SCROLL_FORWARD -> nativeAccessibilityAction(nodeId, ACTION_INCREMENT)
                AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD -> nativeAccessibilityAction(nodeId, ACTION_DECREMENT)
                else -> false
            }
        }

        private fun sendVirtualEvent(virtualViewId: Int, eventType: Int) {
            val event = AccessibilityEvent.obtain(eventType)
            event.packageName = host.context.packageName
            event.className = GpuiActivity::class.java.name
            event.setSource(host, virtualViewId)
            host.parent?.requestSendAccessibilityEvent(host, event)
        }

        private fun classNameForRole(role: String): String {
            return when {
                role.contains("Button") -> android.widget.Button::class.java.name
                role.contains("CheckBox") -> android.widget.CheckBox::class.java.name
                role.contains("Switch") -> android.widget.Switch::class.java.name
                role.contains("TextInput") || role.contains("SearchInput") -> android.widget.EditText::class.java.name
                role.contains("Slider") -> android.widget.SeekBar::class.java.name
                role.contains("Image") -> android.widget.ImageView::class.java.name
                else -> android.widget.TextView::class.java.name
            }
        }
    }

    companion object {
        private const val TAG = "gpui"
        private const val KEYSTORE = "AndroidKeyStore"
        private const val CREDENTIAL_PREFS = "gpui_secure_credentials"
        private const val HOST_ID: Int = View.NO_ID
        private const val ACTION_CLICK = 1
        private const val ACTION_FOCUS = 2
        private const val ACTION_INCREMENT = 3
        private const val ACTION_DECREMENT = 4

        // NativeActivity loads the .so named by `android.app.lib_name` into the
        // *framework* class loader. Subclass-declared `native` methods (the IME /
        // deep-link / accessibility bridges below) are resolved against the *app*
        // class loader, which does not see that load — so JNI throws
        // UnsatisfiedLinkError on first real input. Loading the same library from
        // the app class loader (here, in onCreate) registers it in the right
        // namespace. The name is read from the manifest metadata so this single
        // file works for every generated app regardless of its cdylib name.
        @Volatile
        private var nativeLibLoaded = false

        private val libLock = Any()

        private fun ensureNativeLibLoaded(ctx: Context) {
            if (nativeLibLoaded) return
            synchronized(libLock) {
                if (nativeLibLoaded) return
                var libName: String? = null
                try {
                    // `android.app.lib_name` 声明在本 <activity> 节点下，必须读
                    // ActivityInfo 的 metaData（而非 ApplicationInfo，后者读不到）。
                    val ai = ctx.packageManager.getActivityInfo(
                        (ctx as GpuiActivity).getComponentName(),
                        android.content.pm.PackageManager.GET_META_DATA
                    )
                    libName = ai.metaData?.getString("android.app.lib_name")
                } catch (ignored: Throwable) {
                    // fall through; libName stays null and we surface it below
                }
                if (libName.isNullOrEmpty()) {
                    throw RuntimeException(
                        "GpuiActivity: cannot determine native lib name from " +
                            "android.app.lib_name meta-data"
                    )
                }
                System.loadLibrary(libName)
                nativeLibLoaded = true
            }
        }

        // JNI 桥：@JvmStatic 保证这些 native 方法仍作为 GpuiActivity 的静态方法导出，
        // 与 Java 版同名同 JNI 签名，Rust 侧 GetStaticMethodID 查找不受影响。
        @JvmStatic external fun nativeIsInitialized(): Boolean
        @JvmStatic external fun nativeOnDeepLink(url: String?)
        @JvmStatic external fun nativeCommitText(text: String)
        @JvmStatic external fun nativeKeyEvent(keyCode: Int, action: Int, metaState: Int)
        @JvmStatic external fun nativeSetComposingText(text: String)
        @JvmStatic external fun nativeFinishComposingText()
        @JvmStatic external fun nativeDeleteSurroundingText(before: Int, after: Int)
        @JvmStatic external fun nativeAccessibilitySnapshot(): String?
        @JvmStatic external fun nativeAccessibilityAction(nodeId: Long, action: Int): Boolean
    }
}