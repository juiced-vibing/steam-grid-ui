using System.ComponentModel;
using System.IO;
using System.Runtime.CompilerServices;
using System.Windows.Media;

namespace SteamGridUI;

public abstract class Observable : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    protected void Raise([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}

public sealed class GameItem(Game game) : Observable
{
    private readonly HashSet<Slot> _custom = [.. game.Custom];
    private string? _name = game.Name;

    public string AppId { get; } = game.Appid;
    public uint Id { get; } = uint.Parse(game.Appid);
    public bool Installed { get; } = game.Installed;

    public string? Name
    {
        get => _name;
        set
        {
            _name = value;

            Raise();
            Raise(nameof(Display));
        }
    }

    public string Display => _name ?? $"App {AppId}";

    public string Tags
    {
        get
        {
            var parts = new List<string>();

            if (!Installed) parts.Add("not installed");
            if (_custom.Count > 0) parts.Add("custom: " + string.Join(", ", Enum.GetValues<Slot>().Where(_custom.Contains).Select(s => s.Label())));

            return string.Concat(parts.Select(part => " · " + part));
        }
    }

    public bool Has(Slot slot) => _custom.Contains(slot);

    public void SetHas(Slot slot, bool value)
    {
        if (value ? _custom.Add(slot) : _custom.Remove(slot)) Raise(nameof(Tags));
    }
}

public abstract record Preview
{
    public sealed record Loading : Preview;

    public sealed record Ready(ImageSource Image) : Preview;

    public sealed record Unavailable(string Reason) : Preview;
}

public sealed class AssetItem : Observable
{
    private Preview _preview;

    public AssetItem(Asset asset, ArtKind kind)
    {
        Asset = asset;
        (TileW, TileH) = kind.TileSize();

        var info = new List<string> { $"{asset.Width}×{asset.Height}" };
        if (asset.Style is { } style) info.Add(style.Replace('_', ' '));
        if (Extension.Length > 1) info.Add(Extension[1..]);

        Info = string.Join(" · ", info);
        Author = asset.Author is { } author ? "by " + author : "";

        _preview = asset.Preview is null ? new Preview.Unavailable("animated · no preview") : new Preview.Loading();
    }

    public Asset Asset { get; }
    public string Info { get; }
    public string Author { get; }
    public double TileW { get; }
    public double TileH { get; }

    public string Extension => Path.GetExtension(new Uri(Asset.Url).AbsolutePath);

    public Preview Preview
    {
        get => _preview;
        set
        {
            _preview = value;

            Raise(nameof(Thumb));
            Raise(nameof(Placeholder));
        }
    }

    public ImageSource? Thumb => _preview is Preview.Ready ready ? ready.Image : null;

    public string Placeholder => _preview switch
    {
        Preview.Loading => "loading…",
        Preview.Unavailable unavailable => unavailable.Reason,
        _ => "",
    };
}
