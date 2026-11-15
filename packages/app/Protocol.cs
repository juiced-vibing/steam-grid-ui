using System.Text.Json.Serialization;

namespace SteamGridUI;

public enum ArtKind { Grid, Wide, Hero, Logo, Icon }

public enum Slot { Grid, Wide, Hero, Logo }

public enum Source { Installed, Owned, Shortcut }

public static class ArtKinds
{
    public static Slot? Slot(this ArtKind kind) => kind switch
    {
        ArtKind.Grid => SteamGridUI.Slot.Grid,
        ArtKind.Wide => SteamGridUI.Slot.Wide,
        ArtKind.Hero => SteamGridUI.Slot.Hero,
        ArtKind.Logo => SteamGridUI.Slot.Logo,
        _ => null,
    };

    public static string Label(this ArtKind kind) => kind == ArtKind.Grid ? "portrait" : kind.ToString().ToLowerInvariant();

    public static string Label(this Slot slot) => slot == SteamGridUI.Slot.Grid ? "portrait" : slot.ToString().ToLowerInvariant();

    public static (double Width, double Height) TileSize(this ArtKind kind) => kind switch
    {
        ArtKind.Grid => (160, 240),
        ArtKind.Wide => (230, 108),
        ArtKind.Hero => (372, 120),
        ArtKind.Logo => (220, 130),
        _ => (112, 112),
    };
}

public abstract record Request<TReply>([property: JsonIgnore] string Method);

public sealed record Init() : Request<InitReply>("init");

public sealed record SetApiKey(string Key) : Request<Nothing>("set_api_key");

public sealed record ListGames(string User) : Request<List<Game>>("list_games");

public sealed record FindGame(string Appid, string Name) : Request<List<GameMatch>>("find_game");

public sealed record RememberMatch(string Appid, string Id) : Request<Nothing>("remember_match");

public sealed record ListAssets(GameRef Game, ArtKind Kind, int Page, bool Animated, bool Nsfw, bool Humor, bool NeedName)
    : Request<AssetPage>("list_assets");

public sealed record Thumb(string Url) : Request<PathReply>("thumb");

public sealed record Apply(string User, string Appid, Slot Slot, string Url) : Request<Placed>("apply");

public sealed record Clear(string User, string Appid, Slot Slot) : Request<ClearReply>("clear");

public sealed record SaveAs(string Url, string Dest) : Request<Nothing>("save_as");

public sealed record BackupDir() : Request<PathReply>("backup_dir");

public sealed record Nothing
{
    public static readonly Nothing Value = new();
}

public sealed record InitReply(bool ApiKeySet, SteamState Steam);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "state")]
[JsonDerivedType(typeof(Found), "found")]
[JsonDerivedType(typeof(Missing), "missing")]
public abstract record SteamState
{
    public sealed record Found(string Path, List<User> Users) : SteamState;

    public sealed record Missing(string Reason) : SteamState;
}

public sealed record User(string Id, string Name);

public sealed record Game(string Appid, string? Name, Source Source, List<Slot> Custom);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "by")]
[JsonDerivedType(typeof(Steam), "steam")]
[JsonDerivedType(typeof(Sgdb), "sgdb")]
public abstract record GameRef
{
    public sealed record Steam(string Appid) : GameRef;

    public sealed record Sgdb(string Id) : GameRef;
}

public sealed record GameMatch(string Id, string Name, int? Year)
{
    [JsonIgnore]
    public string Label => Year is { } year ? $"{Name} ({year})" : Name;
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "state")]
[JsonDerivedType(typeof(Known), "known")]
[JsonDerivedType(typeof(Unknown), "unknown")]
public abstract record AssetPage(string? GameName)
{
    public sealed record Known(List<Asset> Items, bool More, string? GameName) : AssetPage(GameName);

    public sealed record Unknown(string? GameName) : AssetPage(GameName);
}

public sealed record Asset(string Url, string? Preview, int Width, int Height, string? Style, string? Author);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "outcome")]
[JsonDerivedType(typeof(Applied), "applied")]
[JsonDerivedType(typeof(Unchanged), "unchanged")]
public abstract record Placed(string Path)
{
    public sealed record Applied(string Path, int Replaced) : Placed(Path);

    public sealed record Unchanged(string Path) : Placed(Path);
}

public sealed record PathReply(string Path);

public sealed record ClearReply(int Removed);

public enum ErrorCode
{
    Unknown,
    BadRequest,
    NoApiKey,
    KeyRejected,
    RateLimited,
    Api,
    Network,
    SteamNotFound,
    UnsupportedFormat,
    UntrustedUrl,
    Io,
    Internal,
}

public sealed class BackendException(ErrorCode code, string message) : Exception(message)
{
    public ErrorCode Code { get; } = code;
}
