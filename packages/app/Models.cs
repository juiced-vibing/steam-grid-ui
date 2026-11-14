using System.IO;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Windows.Media;

namespace SteamGridUI;

public abstract class Observable : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    protected void Raise([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}

public sealed record UserItem(string Id, string Name);

public sealed class GameItem : Observable
{
    private readonly HashSet<string> _custom = [];
    private string _name = "";

    public required string AppId { get; init; }
    public bool Installed { get; init; }

    public string Name
    {
        get => _name;
        set
        {
            _name = value;
            Raise();
            Raise(nameof(Display));
        }
    }

    public bool HasName => _name.Length > 0;
    public string Display => HasName ? _name : $"App {AppId}";

    public string Sub
    {
        get
        {
            var parts = new List<string> { AppId };
            if (!Installed) parts.Add("not installed");
            if (_custom.Count > 0) parts.Add("custom: " + string.Join(", ", Kinds.All.Where(_custom.Contains).Select(Kinds.Label)));
            return string.Join(" · ", parts);
        }
    }

    public bool Has(string kind) => _custom.Contains(kind);

    public void SetHas(string kind, bool value)
    {
        if (value ? _custom.Add(kind) : _custom.Remove(kind)) Raise(nameof(Sub));
    }

    public static GameItem Parse(JsonElement e)
    {
        var g = new GameItem
        {
            AppId = e.GetProperty("appid").GetString()!,
            Installed = e.GetProperty("installed").GetBoolean(),
            Name = e.GetProperty("name").GetString() ?? "",
        };
        foreach (var kind in Kinds.All)
            if (e.GetProperty("has_" + kind).GetBoolean()) g._custom.Add(kind);
        return g;
    }
}

public sealed class AssetItem : Observable
{
    private ImageSource? _thumb;
    private string _placeholder = "loading…";

    public required string Url { get; init; }
    public string? ThumbUrl { get; init; }
    public required string Info { get; init; }
    public required string Author { get; init; }
    public double TileW { get; init; }
    public double TileH { get; init; }

    /// <summary>Animated assets ship a .webm thumbnail, which an Image can't show.</summary>
    public bool NoPreview => ThumbUrl is null || ThumbUrl.EndsWith(".webm", StringComparison.OrdinalIgnoreCase);

    public ImageSource? Thumb
    {
        get => _thumb;
        set { _thumb = value; Raise(); }
    }

    public string Placeholder
    {
        get => _placeholder;
        set { _placeholder = value; Raise(); }
    }

    public static AssetItem Parse(JsonElement e, string kind)
    {
        static string Str(JsonElement e, string name) =>
            e.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.String ? v.GetString()! : "";
        static int Int(JsonElement e, string name) =>
            e.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.Number ? v.GetInt32() : 0;

        var author = e.TryGetProperty("author", out var a) && a.ValueKind == JsonValueKind.Object ? Str(a, "name") : "";
        var info = new List<string> { $"{Int(e, "width")}×{Int(e, "height")}" };
        if (Str(e, "style") is { Length: > 0 } style) info.Add(style.Replace('_', ' '));
        var ext = Path.GetExtension(Str(e, "url")).TrimStart('.');
        if (ext.Length > 0) info.Add(ext);

        var (w, h) = Kinds.TileSize(kind);
        var thumb = Str(e, "thumb");
        var item = new AssetItem
        {
            Url = Str(e, "url"),
            ThumbUrl = thumb.Length > 0 ? thumb : null,
            Info = string.Join(" · ", info),
            Author = author.Length > 0 ? "by " + author : "",
            TileW = w,
            TileH = h,
        };
        if (item.NoPreview) item.Placeholder = "animated · no preview";
        return item;
    }
}

public static class Kinds
{
    /// <summary>Slots Steam reads from userdata/&lt;id&gt;/config/grid. Icons are download-only.</summary>
    public static readonly string[] All = ["grid", "wide", "hero", "logo"];

    public static string Label(string kind) => kind switch
    {
        "grid" => "portrait",
        _ => kind,
    };

    public static (double W, double H) TileSize(string kind) => kind switch
    {
        "grid" => (160, 240),
        "wide" => (230, 108),
        "hero" => (372, 120),
        "logo" => (220, 130),
        _ => (112, 112),
    };
}
