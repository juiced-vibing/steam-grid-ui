using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace SteamGridUI;

internal static class Native
{
    private static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.SnakeCaseLower) },
    };

    [DllImport("sgdb_core", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr sgdb_call(byte[] method, byte[] args);

    [DllImport("sgdb_core", CallingConvention = CallingConvention.Cdecl)]
    private static extern void sgdb_free(IntPtr ptr);

    private static byte[] CString(string s) => Encoding.UTF8.GetBytes(s + "\0");

    public static Task<TReply> Send<TReply>(Request<TReply> request) => Task.Run(() =>
    {
        var args = JsonSerializer.Serialize(request, request.GetType(), Json);
        var ptr = sgdb_call(CString(request.Method), CString(args));

        try
        {
            return Read<TReply>(Marshal.PtrToStringUTF8(ptr) ?? "");
        }

        finally
        {
            sgdb_free(ptr);
        }
    });

    private static TReply Read<TReply>(string response)
    {
        try
        {
            using var doc = JsonDocument.Parse(response);
            var root = doc.RootElement;

            if (!root.GetProperty("ok").GetBoolean()) throw Failure(root.GetProperty("error"));

            if (Nothing.Value is TReply nothing) return nothing;

            return root.GetProperty("data").Deserialize<TReply>(Json)
                ?? throw new BackendException(ErrorCode.Unknown, "The backend sent an empty reply");
        }

        // a reply this build cannot read: a mismatched sgdb_core.dll
        catch (Exception e) when (e is JsonException or KeyNotFoundException or InvalidOperationException)
        {
            throw new BackendException(ErrorCode.Unknown, $"The backend sent a reply this version cannot read ({e.Message})");
        }
    }

    private static BackendException Failure(JsonElement error)
    {
        var message = error.GetProperty("message").GetString() ?? "The request failed";

        try
        {
            return new BackendException(error.GetProperty("code").Deserialize<ErrorCode>(Json), message);
        }

        // a code newer than this build still has a message worth showing
        catch (JsonException)
        {
            return new BackendException(ErrorCode.Unknown, message);
        }
    }
}
