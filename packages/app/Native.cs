using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace SteamGridUI;

/// <summary>An error reported by the Rust backend, already worded for the user.</summary>
public sealed class BackendException(string message) : Exception(message);

/// <summary>Bridge to sgdb_core.dll: one blocking JSON-in/JSON-out call, run off the UI thread.</summary>
internal static class Native
{
    [DllImport("sgdb_core", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr sgdb_call(byte[] method, byte[] args);

    [DllImport("sgdb_core", CallingConvention = CallingConvention.Cdecl)]
    private static extern void sgdb_free(IntPtr ptr);

    private static byte[] CString(string s) => Encoding.UTF8.GetBytes(s + "\0");

    public static Task<JsonElement> Call(string method, object? args = null) => Task.Run(() =>
    {
        var ptr = sgdb_call(CString(method), CString(JsonSerializer.Serialize(args ?? new { })));
        try
        {
            using var doc = JsonDocument.Parse(Marshal.PtrToStringUTF8(ptr) ?? "{}");
            var root = doc.RootElement;
            if (!root.TryGetProperty("ok", out var ok) || !ok.GetBoolean())
            {
                var error = root.TryGetProperty("error", out var e) ? e.GetString() : null;
                throw new BackendException(error ?? "The backend returned an unreadable reply.");
            }
            return root.GetProperty("data").Clone();
        }
        finally
        {
            sgdb_free(ptr);
        }
    });
}
