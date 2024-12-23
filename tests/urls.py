from django.urls import include, path

from . import views

extra = [
    path("<username>/", views.bio, name="user"),
]

urlpatterns = [
    path("", views.home, name="home"),
    path("bio/<username>/", views.bio, name="bio"),
    path("users/", include((extra, "users"))),
    path("members/", include((extra, "users"), "members")),
]
