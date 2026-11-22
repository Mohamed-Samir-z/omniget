import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.content.pm.ProviderInfo
import android.database.Cursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import com.openselena.libreget.DocumentImports
import java.io.File
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config
import org.robolectric.shadows.ShadowContentResolver

class ImportFixtureProvider : ContentProvider() {
    val files = mutableMapOf<String, File>()
    override fun onCreate() = true
    override fun getType(uri: Uri) = "application/octet-stream"
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor? {
        if (uri.lastPathSegment == "denied") throw SecurityException("Document permission expired")
        val file = files[uri.toString()] ?: throw java.io.FileNotFoundException("Document no longer available")
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
    }
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk=[34])
class DocumentImportsTest {
    private lateinit var context: Context
    private lateinit var provider: ImportFixtureProvider
    @Before fun setup() {
        context=RuntimeEnvironment.getApplication()
        provider=ImportFixtureProvider()
        provider.attachInfo(context, ProviderInfo().apply { authority="test.documents" })
        ShadowContentResolver.registerProviderInternal("test.documents", provider)
    }
    private fun document(name:String, bytes:ByteArray):String {
        val file=File.createTempFile("import-fixture", ".bin", context.cacheDir)
        file.writeBytes(bytes)
        val uri="content://test.documents/document/$name"
        provider.files[uri]=file
        return uri
    }
    @Test fun contentProviderTextPreservesUnicodeAndNewlines() {
        val text="\uFEFF# قائمة\nhttps://example.com/1\nhttps://example.com/2|اختيار\n"
        val result=JSONObject(DocumentImports.readBatch(context,document("list%3Aone.txt",text.toByteArray())))
        assertTrue(result.toString(),result.getBoolean("ok"));assertEquals(text,result.getString("value"))
    }
    @Test fun binaryInvalidUtf8DoesNotBecomeReplacementText() {
        val result=JSONObject(DocumentImports.readBatch(context,document("invalid",byteArrayOf(0xc3.toByte(),0x28))))
        assertFalse(result.getBoolean("ok"));assertFalse(result.has("value"))
    }
    @Test fun oversizedTextFailsInsteadOfTruncating() {
        val result=JSONObject(DocumentImports.readBatch(context,document("large",ByteArray(1024*1024+1){65})))
        assertFalse(result.getBoolean("ok"));assertTrue(result.getString("error").contains("too large"))
    }
    @Test fun missingAndExpiredProviderAccessReturnHonestFailure() {
        for(uri in listOf("content://test.documents/document/denied","content://test.documents/document/missing")) {
            assertFalse(JSONObject(DocumentImports.readBatch(context,uri)).getBoolean("ok"))
            assertFalse(JSONObject(DocumentImports.copyTorrent(context,uri)).getBoolean("ok"))
        }
    }
    @Test fun torrentMetadataIsCopiedPrivatelyAndSurvivesSourceRemoval() {
        val bytes="d4:infod4:name1:x6:lengthi0eee".toByteArray()
        val uri=document("metadata.torrent",bytes)
        val result=JSONObject(DocumentImports.copyTorrent(context,uri));assertTrue(result.toString(),result.getBoolean("ok"))
        val target=File(result.getString("value"))
        assertEquals(File(context.filesDir,"torrent-imports").canonicalPath,target.parentFile!!.canonicalPath)
        assertTrue(target.name.endsWith(".torrent"));assertArrayEquals(bytes,target.readBytes())
        provider.files[uri]!!.delete();assertArrayEquals(bytes,target.readBytes())
        assertFalse(target.parentFile!!.listFiles()!!.any{it.name.endsWith(".part")})
    }
    @Test fun emptyAndOversizedTorrentFilesAreRejected() {
        for(bytes in listOf(byteArrayOf(),ByteArray(16*1024*1024+1))) {
            assertFalse(JSONObject(DocumentImports.copyTorrent(context,document("bad",bytes))).getBoolean("ok"))
        }
    }
    @Test fun rawAndFileUriInputsStillWork() {
        val file=File.createTempFile("batch-local", ".txt",context.cacheDir).apply{writeText("https://example.com/video\n")}
        for(path in listOf(file.absolutePath,Uri.fromFile(file).toString())) {
            assertEquals(file.readText(),JSONObject(DocumentImports.readBatch(context,path)).getString("value"))
        }
    }
    @Test fun webAddressesAreNotFetchedAsDocuments() {
        assertFalse(JSONObject(DocumentImports.readBatch(context,"https://example.com/list.txt")).getBoolean("ok"))
        assertFalse(JSONObject(DocumentImports.copyTorrent(context,"javascript:alert(1)")).getBoolean("ok"))
    }
}
