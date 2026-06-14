          if (!Array.isArray(this.agentDrawer._fallbacks)) this.agentDrawer._fallbacks = [];
          this.agentDrawer._fallbacks.push({ provider: fallbackProvider, model: fallbackModel });
          appendedFallback = true;
          configPayload.fallback_models = this.dedupeFallbackModelList(this.agentDrawer._fallbacks, {
            primary_id: (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
            primary_provider: (this.agentDrawer && this.agentDrawer.model_provider) || '',
          });
          this.agentDrawer._fallbacks = configPayload.fallback_models.slice();
        } else if (Array.isArray(this.agentDrawer._fallbacks)) {
          configPayload.fallback_models = this.dedupeFallbackModelList(this.agentDrawer._fallbacks, {
            primary_id: (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
            primary_provider: (this.agentDrawer && this.agentDrawer.model_provider) || '',
          });
          this.agentDrawer._fallbacks = configPayload.fallback_models.slice();
        }

        var configResponse = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/config', configPayload);
        if (configResponse && configResponse.rename_notice) {
          this.addNoticeEvent(configResponse.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }

        if (this.drawerEditingProvider && String(this.drawerNewProviderValue || '').trim()) {
          var previousProviderName = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          var previousModelName = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var resolvedProviderModel = this.resolveProviderScopedModelCatalogOption(
            this.drawerNewProviderValue,
            (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
            this.modelCatalogRows()
          );
          await this.switchAgentModelWithGuards(
            resolvedProviderModel || { id: String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '') },
            {
            agent_id: agentId,
            previous_model: previousModelName,
            previous_provider: previousProviderName
            }
          );
        } else if (this.drawerEditingModel && String(this.drawerNewModelValue || '').trim()) {
          var previousModelNameForModelEdit = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var previousProviderForModelEdit = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          var resolvedDrawerModel = this.resolveModelCatalogOption(
            this.drawerNewModelValue,
            previousProviderForModelEdit,
            this.modelCatalogRows()
          );
          await this.switchAgentModelWithGuards(
            resolvedDrawerModel || { id: String(this.drawerNewModelValue || '').trim() },
            {
              agent_id: agentId,
              previous_model: previousModelNameForModelEdit,
              previous_provider: previousProviderForModelEdit
            }
          );
        }

        this.drawerEditingName = false;
        this.drawerEditingEmoji = false;
        this.drawerEditingModel = false;
        this.drawerEditingProvider = false;
        this.drawerEditingFallback = false;
        this.drawerNewModelValue = '';
        this.drawerNewProviderValue = '';
        this.drawerNewFallbackValue = '';
        InfringToast.success('Agent settings saved');
        await this.syncDrawerAgentAfterChange();
        this.closeAgentDrawer();
      } catch (e) {
        if (appendedFallback) {
          this.agentDrawer._fallbacks = previousFallbacks;
        }
        InfringToast.error('Failed to save agent settings: ' + e.message);
      } finally {
        this.drawerSavePending = false;
        this.drawerConfigSaving = false;
        this.drawerModelSaving = false;
        this.drawerIdentitySaving = false;
      }
    },

    async saveDrawerConfig() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      this.drawerConfigSaving = true;
      try {
        var response = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', this.drawerConfigForm || {});
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }
        InfringToast.success('Config updated');
        await this.syncDrawerAgentAfterChange();
        this.closeAgentDrawer();
      } catch(e) {
        InfringToast.error('Failed to save config: ' + e.message);
      }
      this.drawerConfigSaving = false;
    },

    async saveDrawerIdentity(part) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var payload = {};
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      if (part === 'name') {
        payload.name = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      } else if (part === 'emoji') {
        payload.emoji = String((this.drawerConfigForm && this.drawerConfigForm.emoji) || '').trim();
        if (this.sanitizeAgentEmojiForDisplay) {
          payload.emoji = this.sanitizeAgentEmojiForDisplay(this.agentDrawer || this.currentAgent, payload.emoji);
        }
        if (!payload.emoji) {
          InfringToast.info('The gear icon is reserved for the System thread.');
          this.drawerIdentitySaving = false;
          return;
        }
        payload.avatar_url = '';
        if (this.drawerConfigForm && typeof this.drawerConfigForm === 'object') {
          this.drawerConfigForm.avatar_url = '';
        }
        if (this.agentDrawer && typeof this.agentDrawer === 'object') {
          this.agentDrawer.avatar_url = '';
        }
      } else if (part === 'avatar') {
        payload.avatar_url = String((this.drawerConfigForm && this.drawerConfigForm.avatar_url) || '').trim();
      } else {
        return;
      }
      this.drawerIdentitySaving = true;
      try {
        var response = await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', payload);
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (part === 'name' && payload.name && payload.name !== previousName) {
          this.addAgentRenameNotice(previousName, payload.name);
        }
        if (part === 'name') this.drawerEditingName = false;
        if (part === 'emoji') this.drawerEditingEmoji = false;
        if (part === 'avatar') {
          this.drawerAvatarUploadError = '';
          this.drawerAvatarUrlPickerOpen = false;
          this.drawerAvatarUrlDraft = '';
        }
        InfringToast.success(
          part === 'name'
            ? 'Name updated'
            : (part === 'emoji' ? 'Emoji updated' : 'Avatar updated')
        );
        await this.syncDrawerAgentAfterChange();
        this.closeAgentDrawer();
      } catch(e) {
        InfringToast.error('Failed to save ' + part + ': ' + e.message);
      }
      this.drawerIdentitySaving = false;
    },

    async changeDrawerModel() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewModelValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var resolvedDrawerModel = this.resolveModelCatalogOption(
          this.drawerNewModelValue,
          previousProvider,
          this.modelCatalogRows()
        );
        var resp = await this.switchAgentModelWithGuards(
          resolvedDrawerModel || { id: String(this.drawerNewModelValue || '').trim() },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        var providerInfo = (resp && resp.provider) ? ' (provider: ' + resp.provider + ')' : '';
        InfringToast.success('Model changed' + providerInfo + ' (memory reset)');
        this.drawerEditingModel = false;
        this.drawerNewModelValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change model: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async changeDrawerProvider() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewProviderValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var resolvedProviderModel = this.resolveProviderScopedModelCatalogOption(
          this.drawerNewProviderValue,
          previousModel || (this.agentDrawer && this.agentDrawer.model_name) || '',
          this.modelCatalogRows()
        );
        var resp = await this.switchAgentModelWithGuards(
          resolvedProviderModel || { id: String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '') },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        InfringToast.success('Provider changed to ' + (resp && resp.provider ? resp.provider : String(this.drawerNewProviderValue || '').trim()));
        this.drawerEditingProvider = false;
        this.drawerNewProviderValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change provider: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async addDrawerFallback() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewFallbackValue || '').trim()) return;
      var parts = String(this.drawerNewFallbackValue || '').trim().split('/');
      var provider = parts.length > 1 ? parts[0] : this.agentDrawer.model_provider;
      var model = parts.length > 1 ? parts.slice(1).join('/') : parts[0];
      if (!this.agentDrawer._fallbacks) this.agentDrawer._fallbacks = [];
      var previousFallbacks = this.agentDrawer._fallbacks.slice();
      var nextFallbacks = this.dedupeFallbackModelList(
        this.agentDrawer._fallbacks.concat([{ provider: provider, model: model }]),
        {
          primary_id: (this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || '',
          primary_provider: (this.agentDrawer && this.agentDrawer.model_provider) || '',
        }
      );
      if (nextFallbacks.length === this.agentDrawer._fallbacks.length) {
        InfringToast.info('Fallback already exists or matches the primary model');
        return;
      }
      this.agentDrawer._fallbacks = nextFallbacks;
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        var latestFallback = this.agentDrawer._fallbacks[this.agentDrawer._fallbacks.length - 1] || {};
        InfringToast.success('Fallback added: ' + String((latestFallback.provider || provider) || '').trim() + '/' + String((latestFallback.model || model) || '').trim());
        this.drawerEditingFallback = false;
        this.drawerNewFallbackValue = '';
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        this.agentDrawer._fallbacks = previousFallbacks;
      }
    },

    async removeDrawerFallback(idx) {
      if (!this.agentDrawer || !this.agentDrawer.id || !Array.isArray(this.agentDrawer._fallbacks)) return;
      var removed = this.agentDrawer._fallbacks.splice(idx, 1);
      try {
        await InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(this.agentDrawer.id) + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        InfringToast.success('Fallback removed');
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        if (removed && removed.length) this.agentDrawer._fallbacks.splice(idx, 0, removed[0]);
      }
    },
