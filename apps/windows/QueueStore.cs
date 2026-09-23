using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Microsoft.UI.Dispatching;
using ReviewRadar.Windows.Models;

namespace ReviewRadar.Windows;

public sealed class QueueStore : INotifyPropertyChanged
{
    private static readonly JsonSerializerOptions JsonOptions = new() { PropertyNameCaseInsensitive = true };
    private readonly IOsIntegration _osIntegration;
    private readonly DispatcherQueueTimer _refreshTimer;
    private bool _refreshing;
    private PullRequestCard? _selectedCard;
    private string _statusText = "Loading workspace…";

    public QueueStore(IOsIntegration? osIntegration = null)
    {
        _osIntegration = osIntegration ?? new WindowsOsIntegration();
        _refreshTimer = DispatcherQueue.GetForCurrentThread().CreateTimer();
        _refreshTimer.Interval = TimeSpan.FromMinutes(5);
        _refreshTimer.Tick += (_, _) => _ = RefreshAsync();
    }

    public ObservableCollection<PullRequestCard> Cards { get; } = [];

    public PullRequestCard? SelectedCard
    {
        get => _selectedCard;
        set => SetField(ref _selectedCard, value);
    }

    public string StatusText
    {
        get => _statusText;
        private set => SetField(ref _statusText, value);
    }

    public async Task StartAsync()
    {
        await RefreshAsync();
        _refreshTimer.Start();
    }

    public async Task RefreshAsync()
    {
        if (_refreshing)
        {
            return;
        }

        _refreshing = true;
        try
        {
            await LoadProjectionAsync(recordAttention: true);
            if (!string.IsNullOrEmpty(Environment.GetEnvironmentVariable("REVIEW_RADAR_SKIP_COLLECTION")))
            {
                return;
            }

            try
            {
                StatusText = Cards.Count == 0 ? "Refreshing GitHub…" : "Refreshing GitHub in the background…";
                await RunHelperAsync(Helper("REVIEW_RADAR_COLLECTOR_COMMAND", "review-radar-github"), [
                    "--database", CaptureDatabase,
                ]);
            }
            catch (Exception error)
            {
                StatusText = Cards.Count == 0
                    ? $"Could not refresh GitHub: {error.Message}"
                    : $"Showing cached pull requests after refresh failure: {error.Message}";
                return;
            }

            await LoadProjectionAsync(recordAttention: true);
        }
        finally
        {
            _refreshing = false;
        }
    }

    private async Task LoadProjectionAsync(bool recordAttention)
    {
        try
        {
            if (Cards.Count > 0)
            {
                StatusText = "Updating cached workspace…";
            }

            var output = await RunHelperAsync(Helper("REVIEW_RADAR_QUEUE_COMMAND", "review-radar-queue"), [
                "--database", CaptureDatabase,
                "--state-database", StateDatabase,
                "--view", "tailored",
                "--ranking", "tailored",
                "--record-attention", recordAttention ? "true" : "false",
            ]);
            var response = JsonSerializer.Deserialize<QueueResponse>(output.StandardOutput, JsonOptions)
                ?? throw new InvalidDataException("The queue helper did not return a JSON response.");
            if (response.SchemaVersion != 1 || response.View != "tailored" || response.Ranking != "tailored")
            {
                throw new InvalidDataException("The queue response did not match the supported Tailored projection.");
            }

            Apply(response);
            StatusText = $"Updated {response.CapturedAt}";
        }
        catch (Exception error)
        {
            StatusText = Cards.Count == 0
                ? $"Could not load the workspace: {error.Message}"
                : $"Showing cached pull requests after projection failure: {error.Message}";
        }
    }

    private void Apply(QueueResponse response)
    {
        var selectedId = SelectedCard?.Id;
        Cards.Clear();
        foreach (var card in response.PullRequests)
        {
            Cards.Add(card);
        }

        SelectedCard = selectedId is null ? null : Cards.FirstOrDefault(card => card.Id == selectedId);
    }

    private string CaptureDatabase => Override("REVIEW_RADAR_CAPTURE_DATABASE")
        ?? Path.Combine(_osIntegration.ApplicationDataDirectory, "review-radar.sqlite3");

    private string StateDatabase => Override("REVIEW_RADAR_STATE_DATABASE")
        ?? Path.Combine(_osIntegration.ApplicationDataDirectory, "review-radar-state.sqlite3");

    private static string? Override(string name)
    {
        var value = Environment.GetEnvironmentVariable(name);
        return string.IsNullOrWhiteSpace(value) ? null : value;
    }

    private static string Helper(string environmentName, string fallback)
    {
        var configured = Override(environmentName);
        if (configured is not null)
        {
            return configured;
        }

        var embedded = Path.Combine(AppContext.BaseDirectory, "Helpers", $"{fallback}.exe");
        return File.Exists(embedded) ? embedded : fallback;
    }

    private static async Task<HelperOutput> RunHelperAsync(string fileName, IReadOnlyList<string> arguments)
    {
        var startInfo = new ProcessStartInfo(fileName)
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
        foreach (var argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }

        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException($"Could not start {Path.GetFileName(fileName)}.");
        var stdout = process.StandardOutput.ReadToEndAsync();
        var stderr = process.StandardError.ReadToEndAsync();
        await Task.WhenAll(stdout, stderr, process.WaitForExitAsync());
        if (process.ExitCode != 0)
        {
            var detail = stderr.Result.Trim();
            throw new InvalidOperationException(string.IsNullOrEmpty(detail)
                ? $"{Path.GetFileName(fileName)} exited with code {process.ExitCode}."
                : detail);
        }

        return new HelperOutput(stdout.Result, stderr.Result);
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void SetField<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return;
        }

        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    private sealed record HelperOutput(string StandardOutput, string StandardError);
}
