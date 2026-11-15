using System.Collections.ObjectModel;
using System.Diagnostics;
using System.IO;
using System.Windows;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Win32;

namespace SteamGridUI;

public partial class MainWindow
{
    private readonly ObservableCollection<AssetItem> _assets = [];
    private readonly SemaphoreSlim _thumbGate = new(6);

    // bumped on every fresh listing, so replies to an older one are dropped
    private int _generation;
    private int _page;

    private AssetItem? Selected => AssetsList.SelectedItem as AssetItem;

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
        if (game.Source != Source.Shortcut) return new GameRef.Steam(game.AppId);

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
