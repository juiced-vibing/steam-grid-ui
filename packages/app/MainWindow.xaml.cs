using System.Collections;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Win32;

namespace SteamGridUI;

public partial class MainWindow : Window
{
    private sealed record Target(User User, GameItem Game, ArtKind Kind);

    private static readonly IComparer ById = Comparer<GameItem>.Create((a, b) => a.Id.CompareTo(b.Id));

    private readonly ObservableCollection<AssetItem> _assets = [];
    private readonly SemaphoreSlim _thumbGate = new(6);

    private List<GameItem> _games = [];

    // bumped on every fresh listing, so replies to an older one are dropped
    private int _generation;
    private int _page;

    public MainWindow()
    {
        InitializeComponent();

        AssetsList.ItemsSource = _assets;

        // wired here, not in XAML: they would fire while the window is still being built
        UserBox.SelectionChanged += (_, _) => Run(LoadGames);
        GamesList.SelectionChanged += (_, _) => Run(() => LoadAssets(append: false));
        KindBox.SelectionChanged += (_, _) => Run(() => LoadAssets(append: false));
        AssetsList.SelectionChanged += (_, _) => UpdateButtons();
        SearchBox.TextChanged += (_, _) => RefreshFilter();
        SortBox.SelectionChanged += (_, _) => ApplySort();
        MatchBox.SelectionChanged += (_, _) => Run(PickMatch);
    }

    private ArtKind Kind => (ArtKind)((ComboBoxItem)KindBox.SelectedItem).Tag;

    private Target? Current =>
        UserBox.SelectedItem is User user && GamesList.SelectedItem is GameItem game ? new Target(user, game, Kind) : null;

    private AssetItem? Selected => AssetsList.SelectedItem as AssetItem;

    private void SetStatus(string text) => Status.Text = text;

    private void Report(Exception error)
    {
        switch (error)
        {
            case BackendException { Code: ErrorCode.NoApiKey } e:
                SetStatus(e.Message);
                AskApiKey();
                break;

            case BackendException e:
                SetStatus(e.Message);
                break;

            case DllNotFoundException:
                SetStatus("sgdb_core.dll is missing; it belongs next to SteamGridUI.exe");
                break;

            default:
                SetStatus($"Unexpected error: {error.Message}");
                break;
        }
    }

    private async void Run(Func<Task> work)
    {
        try
        {
            await work();
        }

        catch (Exception e)
        {
            Report(e);
        }
    }

    private void RunAction(Func<Task> work) => Run(async () =>
    {
        Actions.IsEnabled = false;

        try
        {
            await work();
        }

        finally
        {
            Actions.IsEnabled = true;
            UpdateButtons();
        }
    });

    private void OnLoaded(object sender, RoutedEventArgs e) => Run(async () =>
    {
        var init = await Native.Send(new Init());

        switch (init.Steam)
        {
            case SteamState.Missing missing:
                SetStatus(missing.Reason);
                break;

            case SteamState.Found { Users.Count: 0 }:
                SetStatus("No Steam accounts found. Sign in to Steam once, then restart this app.");
                break;

            case SteamState.Found found:
                UserBox.ItemsSource = found.Users;
                UserBox.SelectedIndex = 0; // most recently used account; loads its games
                break;
        }

        if (!init.ApiKeySet) AskApiKey();
    });

    private async Task LoadGames()
    {
        if (UserBox.SelectedItem is not User user) return;

        SetStatus("Reading Steam library…");
        var games = await Native.Send(new ListGames(user.Id));
        if (!ReferenceEquals(user, UserBox.SelectedItem)) return;

        _games = games.Select(game => new GameItem(game)).ToList();
        GamesList.ItemsSource = _games;
        CollectionViewSource.GetDefaultView(_games).Filter = MatchesFilter;
        ApplySort();

        SetStatus($"{_games.Count(g => g.Installed)} installed games, {_games.Count} known to this account.");
    }

    private bool MatchesFilter(object item)
    {
        var game = (GameItem)item;
        if (InstalledOnly.IsChecked == true && !game.Installed) return false;

        var query = SearchBox.Text.Trim();
        return query.Length == 0
            || game.Display.Contains(query, StringComparison.OrdinalIgnoreCase)
            || game.AppId.StartsWith(query, StringComparison.Ordinal);
    }

    // the core lists by name, so only the other order needs a comparer
    private void ApplySort() =>
        ((ListCollectionView)CollectionViewSource.GetDefaultView(_games)).CustomSort = SortBox.SelectedIndex == 1 ? ById : null;

    private void RefreshFilter() => CollectionViewSource.GetDefaultView(_games).Refresh();

    private void OnInstalledOnlyClick(object sender, RoutedEventArgs e) => RefreshFilter();

    private void OnApiKey(object sender, RoutedEventArgs e) => AskApiKey();

    private void AskApiKey()
    {
        if (new ApiKeyWindow { Owner = this }.ShowDialog() != true) return;

        SetStatus("API key saved.");
        Run(() => LoadAssets(append: false));
    }

    private void OnOptionClick(object sender, RoutedEventArgs e) => Run(() => LoadAssets(append: false));

    private void OnMore(object sender, RoutedEventArgs e) => Run(() => LoadAssets(append: true));

    private void ShowMatches(GameItem? game)
    {
        MatchBox.ItemsSource = game?.Matches;
        MatchBox.SelectedItem = game?.Match;
        MatchBox.Visibility = game?.Matches is { Count: > 0 } ? Visibility.Visible : Visibility.Collapsed;
    }

    private async Task PickMatch()
    {
        if (Current is not { Game: var game } || MatchBox.SelectedItem is not GameMatch match || match == game.Match) return;

        game.Match = match;
        await Native.Send(new RememberMatch(game.AppId, match.Id));
        await LoadAssets(append: false);
    }

    // a steam game is known by its appid; a shortcut only has a name to search for
    private static async Task<GameRef?> Resolve(GameItem game)
    {
        if (game.Origin == Origin.Steam) return new GameRef.Steam(game.AppId);

        game.Matches ??= await Native.Send(new FindGame(game.AppId, game.Display));
        game.Match ??= game.Matches.FirstOrDefault();

        return game.Match is { } match ? new GameRef.Sgdb(match.Id) : null;
    }

    private async Task LoadAssets(bool append)
    {
        MoreBtn.IsEnabled = false;

        if (!append)
        {
            _generation++;
            _page = 0;
            _assets.Clear();
            ShowMatches(Current?.Game);
        }

        UpdateButtons();
        if (Current is not { } target) return;

        var generation = _generation;
        var page = append ? _page + 1 : 0;
        SetStatus($"Loading {target.Game.Display}…");

        AssetPage reply;
        try
        {
            var game = await Resolve(target.Game);
            if (generation != _generation) return;

            ShowMatches(target.Game);
            if (game is null)
            {
                SetStatus($"SteamGridDB has no game named {target.Game.Display}. Rename the shortcut in Steam to search for another name.");
                return;
            }

            reply = await Native.Send(new ListAssets(
                game,
                target.Kind,
                page,
                Animated: AnimatedBox.IsChecked == true,
                Nsfw: NsfwBox.IsChecked == true,
                Humor: HumorBox.IsChecked == true,
                NeedName: target.Game.Name is null));
        }

        // the list has moved on: its failure is no longer news
        catch (BackendException) when (generation != _generation)
        {
            return;
        }

        if (generation != _generation) return;

        if (reply.GameName is { } name) target.Game.Name = name;

        switch (reply)
        {
            case AssetPage.Unknown:
                SetStatus($"SteamGridDB has no entry for appid {target.Game.AppId}.");
                break;

            case AssetPage.Known known:
                _page = page;
                MoreBtn.IsEnabled = known.More;

                foreach (var asset in known.Items)
                {
                    var item = new AssetItem(asset, target.Kind);
                    _assets.Add(item);
                    _ = LoadThumb(item, generation);
                }

                SetStatus(_assets.Count == 0
                    ? $"No matching artwork for {target.Game.Display}. Try other filters."
                    : $"{target.Game.Display}: {_assets.Count} results{(known.More ? ", more available" : "")}.");
                break;
        }
    }

    private async Task LoadThumb(AssetItem item, int generation)
    {
        if (item.Asset.Preview is not { } url) return;

        await _thumbGate.WaitAsync();

        try
        {
            if (generation != _generation) return;

            var thumb = await Native.Send(new Thumb(url));
            item.Preview = new Preview.Ready(await Task.Run(() => Decode(thumb.Path)));
        }

        // one tile failing (a download, an image WPF cannot decode) must not disturb the list
        catch (Exception e) when (e is BackendException or NotSupportedException or IOException)
        {
            item.Preview = new Preview.Unavailable("preview unavailable");
        }

        finally
        {
            _thumbGate.Release();
        }
    }

    private static ImageSource Decode(string path)
    {
        var image = new BitmapImage();

        image.BeginInit();
        image.CacheOption = BitmapCacheOption.OnLoad; // read fully now; don't keep the file locked
        image.UriSource = new Uri(path);
        image.EndInit();
        image.Freeze();

        return image;
    }

    private void UpdateButtons()
    {
        var slot = Kind.Slot();

        ApplyBtn.IsEnabled = slot is not null && Selected is not null && Current is not null;
        ApplyBtn.ToolTip = slot is null ? "Steam has no custom icon slot for its own games. Use Save as… instead." : null;
        SaveBtn.IsEnabled = Selected is not null && Current is not null;
        RevertBtn.IsEnabled = slot is { } s && Current is { } target && target.Game.Has(s);
    }

    private void OnAssetDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if (ApplyBtn.IsEnabled && Actions.IsEnabled) OnApply(sender, e);
    }

    private void OnApply(object sender, RoutedEventArgs e)
    {
        if (Current is not { } target || Selected is not { } item || target.Kind.Slot() is not { } slot) return;

        RunAction(async () =>
        {
            SetStatus("Downloading…");
            var placed = await Native.Send(new Apply(target.User.Id, target.Game.AppId, slot, item.Asset.Url));

            target.Game.SetHas(slot, true);
            SetStatus(placed switch
            {
                Placed.Unchanged => $"{target.Game.Display} already uses this image.",
                Placed.Applied { Replaced: > 0 } => $"Applied to {target.Game.Display}; the previous image is in the backup folder. Restart Steam if the library doesn't pick it up.",
                Placed.Applied => $"Applied to {target.Game.Display}. Restart Steam if the library doesn't pick it up.",
                _ => throw new UnreachableException(),
            });
        });
    }

    private void OnRevert(object sender, RoutedEventArgs e)
    {
        if (Current is not { } target || target.Kind.Slot() is not { } slot) return;

        RunAction(async () =>
        {
            await Native.Send(new Clear(target.User.Id, target.Game.AppId, slot));
            target.Game.SetHas(slot, false);

            var backups = await Native.Send(new BackupDir());
            SetStatus($"Removed custom {slot.Label()} art for {target.Game.Display}. The old file is in {backups.Path}.");
        });
    }

    private void OnSaveAs(object sender, RoutedEventArgs e)
    {
        if (Current is not { } target || Selected is not { } item) return;

        var safeName = string.Concat(target.Game.Display.Select(c => Path.GetInvalidFileNameChars().Contains(c) ? '_' : c));
        var dialog = new SaveFileDialog
        {
            FileName = $"{safeName} {target.Kind.Label()}{item.Extension}",
            Filter = $"Image (*{item.Extension})|*{item.Extension}|All files|*.*",
        };

        if (dialog.ShowDialog(this) != true) return;

        RunAction(async () =>
        {
            SetStatus("Downloading…");
            await Native.Send(new SaveAs(item.Asset.Url, dialog.FileName));
            SetStatus($"Saved {dialog.FileName}");
        });
    }
}
