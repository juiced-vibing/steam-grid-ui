using System.Text.Json.Serialization;

namespace SteamGridUI;

/// <summary>What SteamGridDB can be asked for</summary>
public enum ArtKind { Grid, Wide, Hero, Logo, Icon }

/// <summary>The kinds Steam has a custom-art slot for; there is none for icons</summary>
public enum Slot { Grid, Wide, Hero, Logo }

public static class ArtKinds
{
    /// <summary>The Steam slot a kind is applied to, or null when it can only be saved to disk</summary>
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

/// <summary>A request to the backend that knows the type of its reply</summary>
public abstract record Request<TReply>([property: JsonIgnore] string Method);

public sealed record Init() : Request<InitReply>("init");

public sealed record SetApiKey(string Key) : Request<Nothing>("set_api_key");

public sealed record ListGames(string User) : Request<List<Game>>("list_games");

public sealed record ListAssets(string Appid, ArtKind Kind, int Page, bool Animated, bool Nsfw, bool Humor, bool NeedName)
    : Request<AssetPage>("list_assets");

public sealed record Thumb(string Url) : Request<PathReply>("thumb");

public sealed record Apply(string User, string Appid, Slot Slot, string Url) : Request<Placed>("apply");

public sealed record Clear(string User, string Appid, Slot Slot) : Request<ClearReply>("clear");

public sealed record SaveAs(string Url, string Dest) : Request<Nothing>("save_as");

public sealed record BackupDir() : Request<PathReply>("backup_dir");

/// <summary>The reply of a request that only succeeds or fails</summary>
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

public sealed record Game(string Appid, string? Name, bool Installed, List<Slot> Custom);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "state")]
[JsonDerivedType(typeof(Known), "known")]
[JsonDerivedType(typeof(Unknown), "unknown")]
public abstract record AssetPage(string? GameName)
{
    public sealed record Known(List<Asset> Items, bool More, string? GameName) : AssetPage(GameName);

    /// <summary>SteamGridDB has no entry for the appid</summary>
    public sealed record Unknown(string? GameName) : AssetPage(GameName);
}

/// <summary>One piece of artwork; Preview is null for animated artwork, which has no still image</summary>
public sealed record Asset(string Url, string? Preview, int Width, int Height, string? Style, string? Author);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "outcome")]
[JsonDerivedType(typeof(Applied), "applied")]
[JsonDerivedType(typeof(Unchanged), "unchanged")]
public abstract record Placed(string Path)
{
    public sealed record Applied(string Path, int Replaced) : Placed(Path);

    /// <summary>The slot already held this exact image</summary>
    public sealed record Unchanged(string Path) : Placed(Path);
}

public sealed record PathReply(string Path);

public sealed record ClearReply(int Removed);

/// <summary>Why a request failed; mirrors Error::code in sgdb_core</summary>
public enum ErrorCode
{
    /// <summary>A code this build does not know, or a reply that could not be read</summary>
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

/// <summary>A failed request: a code to act on and a message already worded for the user</summary>
public sealed class BackendException(ErrorCode code, string message) : Exception(message)
{
    public ErrorCode Code { get; } = code;
}
